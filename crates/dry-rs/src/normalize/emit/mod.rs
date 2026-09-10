//! Convert `syn` syntax into [`NormNode`] trees.

mod expr;
mod ops;

use syn::{Block, Pat, Stmt};

use super::placeholders::PlaceholderMap;
use super::tree::NormNode;

#[doc(inline)]
pub use expr::emit_expr;

/// Emits a normalized tree for a function or method body.
pub fn emit_block(block: &Block, placeholders: &mut PlaceholderMap) -> NormNode {
    let children = block.stmts.iter().map(|stmt| emit_stmt(stmt, placeholders)).collect();
    NormNode::branch("block", children)
}

fn emit_stmt(stmt: &Stmt, placeholders: &mut PlaceholderMap) -> NormNode {
    match stmt {
        Stmt::Local(local) => emit_local(local, placeholders),
        Stmt::Expr(expr, _) => emit_expr(expr, placeholders),
        other => emit_item_or_macro(other),
    }
}

fn emit_item_or_macro(stmt: &Stmt) -> NormNode {
    classify_item_macro(stmt).unwrap_or_else(stmt_other_node)
}

fn classify_item_macro(stmt: &Stmt) -> Option<NormNode> {
    match stmt {
        Stmt::Item(_) => Some(NormNode::leaf("item")),
        Stmt::Macro(mac) => Some(NormNode::leaf(format!(
            "macro:{}",
            mac.mac.path.segments.len()
        ))),
        _ => None,
    }
}

fn stmt_other_node() -> NormNode {
    NormNode::leaf("stmt_other")
}

fn emit_local(local: &syn::Local, placeholders: &mut PlaceholderMap) -> NormNode {
    let mut children = vec![emit_pat(&local.pat, placeholders)];
    if let Some(init) = &local.init {
        children.push(emit_expr(&init.expr, placeholders));
    }
    NormNode::branch("local", children)
}

pub(super) fn emit_pat(pat: &Pat, placeholders: &mut PlaceholderMap) -> NormNode {
    if let Some(node) = try_emit_simple_pat(pat, placeholders) {
        return node;
    }
    if let Some(node) = try_emit_compound_pat(pat, placeholders) {
        return node;
    }
    NormNode::leaf("pat_other")
}

fn try_emit_simple_pat(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match pat {
        Pat::Ident(ident) => NormNode::leaf(placeholders.placeholder(&ident.ident.to_string())),
        Pat::Type(ty) => NormNode::branch("pat_type", vec![emit_pat(&ty.pat, placeholders)]),
        Pat::Wild(_) => NormNode::leaf("pat_wild"),
        _ => return None,
    })
}

fn try_emit_compound_pat(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match pat {
        Pat::Tuple(tuple) => {
            let children = tuple.elems.iter().map(|elem| emit_pat(elem, placeholders)).collect();
            NormNode::branch("pat_tuple", children)
        }
        Pat::Struct(strct) => {
            let children =
                strct.fields.iter().map(|field| emit_pat(&field.pat, placeholders)).collect();
            NormNode::branch("pat_struct", children)
        }
        Pat::TupleStruct(ts) => {
            let children = ts.elems.iter().map(|elem| emit_pat(elem, placeholders)).collect();
            NormNode::branch("pat_tuple_struct", children)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::placeholders::PlaceholderMap;
    use syn::parse_quote;

    #[test]
    fn emit_patterns_and_stmts() {
        let mut placeholders = PlaceholderMap::default();
        let block: syn::Block = parse_quote! {{
            let x = 1;
            let (a, b) = (1, 2);
            let Foo { y } = foo;
            let Bar(z) = bar;
            let _ = 0;
            let typed: i32 = 1;
            fn nested() {}
            assert!(true);
            println!("{}", 1);
        }};
        let node = emit_block(&block, &mut placeholders);
        assert_eq!(node.label, "block");
        assert!(node.children.iter().any(|c| c.label.starts_with("macro:")));
        assert!(node.children.iter().any(|c| c.label == "item"));
        let wild: syn::Pat = parse_quote!(_);
        assert_eq!(emit_pat(&wild, &mut placeholders).label, "pat_wild");
        let other: syn::Pat = parse_quote!(1..=2);
        assert_eq!(emit_pat(&other, &mut placeholders).label, "pat_other");
        let local: syn::Stmt = parse_quote!(let x = 1;);
        assert!(classify_item_macro(&local).is_none());
        assert_eq!(emit_item_or_macro(&local).label, "stmt_other");
    }
}
