//! Convert `syn` syntax into [`NormNode`] trees.

mod expr;
mod lit;
mod mac;
mod ops;
mod pat;
mod shared;

use syn::{Block, Stmt};

use super::placeholders::PlaceholderMap;
use dry_core::NormNode;

#[doc(inline)]
pub use expr::emit_expr;
pub(super) use lit::lit_label;
use mac::emit_macro;
pub(super) use pat::emit_pat;

/// Emits a normalized tree for a function or method body.
pub fn emit_block(block: &Block, placeholders: &mut PlaceholderMap) -> NormNode {
    let children = block.stmts.iter().map(|stmt| emit_stmt(stmt, placeholders)).collect();
    NormNode::branch("block", children)
}

fn emit_stmt(stmt: &Stmt, placeholders: &mut PlaceholderMap) -> NormNode {
    match stmt {
        Stmt::Local(local) => emit_local(local, placeholders),
        Stmt::Expr(expr, _) => emit_expr(expr, placeholders),
        other => emit_item_or_macro(other, placeholders),
    }
}

fn emit_item_or_macro(stmt: &Stmt, placeholders: &mut PlaceholderMap) -> NormNode {
    classify_item_macro(stmt, placeholders).unwrap_or_else(stmt_other_node)
}

fn classify_item_macro(stmt: &Stmt, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    match stmt {
        Stmt::Item(_) => Some(NormNode::leaf("item")),
        Stmt::Macro(mac) => Some(emit_macro(&mac.mac, placeholders)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::placeholders::PlaceholderMap;
    use syn::parse_quote;

    #[test]
    fn emit_block_covers_macros_and_items() {
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
        assert!(node.children.iter().any(|c| c.label.starts_with("macro:assert:")));
        assert!(node.children.iter().any(|c| c.label.starts_with("macro:println:")));
        assert!(node.children.iter().any(|c| c.label == "item"));
        let local: syn::Stmt = parse_quote!(let x = 1;);
        assert!(classify_item_macro(&local, &mut placeholders).is_none());
        assert_eq!(
            emit_item_or_macro(&local, &mut placeholders).label,
            "stmt_other"
        );
    }
}
