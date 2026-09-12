//! Macro emission: selective expand for an allowlist, else token-tree shape.

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::{Expr, Macro, MacroDelimiter, Token};

use crate::normalize::placeholders::PlaceholderMap;
use dry_core::NormNode;

/// Emits a normalized tree for a macro, expanding allowlisted invocations when
/// `emit_expr` can normalize argument expressions.
#[must_use]
pub fn emit_macro(
    mac: &Macro,
    placeholders: &mut PlaceholderMap,
    emit_expr: fn(&Expr, &mut PlaceholderMap) -> NormNode,
) -> NormNode {
    let name = macro_name(mac);
    if is_expand_allowlisted(&name)
        && let Some(node) = try_expand_macro(mac, &name, placeholders, emit_expr)
    {
        return node;
    }
    emit_macro_tokens(mac, &name, placeholders)
}

fn emit_macro_tokens(mac: &Macro, name: &str, placeholders: &mut PlaceholderMap) -> NormNode {
    let delim = delimiter_label(&mac.delimiter);
    let label = format!("macro:{name}:{delim}");
    let children = emit_token_stream(mac.tokens.clone(), placeholders);
    if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    }
}

fn try_expand_macro(
    mac: &Macro,
    name: &str,
    placeholders: &mut PlaceholderMap,
    emit_expr: fn(&Expr, &mut PlaceholderMap) -> NormNode,
) -> Option<NormNode> {
    let exprs = parse_macro_exprs(mac.tokens.clone())?;
    let delim = delimiter_label(&mac.delimiter);
    let label = format!("macro_expand:{name}:{delim}");
    let children: Vec<NormNode> = exprs.iter().map(|expr| emit_expr(expr, placeholders)).collect();
    Some(if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    })
}

fn parse_macro_exprs(tokens: TokenStream) -> Option<Vec<Expr>> {
    use syn::parse::Parser;
    Punctuated::<Expr, Token![,]>::parse_terminated
        .parse2(tokens)
        .ok()
        .map(|list| list.into_iter().collect())
}

fn is_expand_allowlisted(name: &str) -> bool {
    matches!(
        name,
        "vec"
            | "assert"
            | "assert_eq"
            | "assert_ne"
            | "format"
            | "write"
            | "writeln"
            | "println"
            | "eprintln"
            | "dbg"
            | "matches"
    )
}

fn macro_name(mac: &Macro) -> String {
    mac.path
        .segments
        .last()
        .map_or_else(|| "unknown".to_owned(), |seg| seg.ident.to_string())
}

const fn delimiter_label(delimiter: &MacroDelimiter) -> &'static str {
    match delimiter {
        MacroDelimiter::Paren(_) => "paren",
        MacroDelimiter::Brace(_) => "brace",
        MacroDelimiter::Bracket(_) => "bracket",
    }
}

const fn group_label(delimiter: Delimiter) -> &'static str {
    match delimiter {
        Delimiter::Parenthesis => "tt_paren",
        Delimiter::Brace => "tt_brace",
        Delimiter::Bracket => "tt_bracket",
        Delimiter::None => "tt_none",
    }
}

fn emit_token_stream(stream: TokenStream, placeholders: &mut PlaceholderMap) -> Vec<NormNode> {
    stream.into_iter().map(|tree| emit_token_tree(tree, placeholders)).collect()
}

fn emit_token_tree(tree: TokenTree, placeholders: &mut PlaceholderMap) -> NormNode {
    match tree {
        TokenTree::Group(group) => emit_group(&group, placeholders),
        TokenTree::Ident(ident) => NormNode::leaf(placeholders.placeholder(&ident.to_string())),
        TokenTree::Literal(literal) => NormNode::leaf(literal_label(literal)),
        TokenTree::Punct(punct) => NormNode::leaf(format!("punct:{}", punct.as_char())),
    }
}

fn emit_group(group: &proc_macro2::Group, placeholders: &mut PlaceholderMap) -> NormNode {
    let label = group_label(group.delimiter());
    let children = emit_token_stream(group.stream(), placeholders);
    if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    }
}

fn literal_label(literal: proc_macro2::Literal) -> &'static str {
    let mut stream = TokenStream::new();
    stream.extend(std::iter::once(TokenTree::Literal(literal)));
    label_from_lit_stream(stream)
}

fn label_from_lit_stream(stream: TokenStream) -> &'static str {
    syn::parse2::<syn::Lit>(stream).map_or("lit_other", |lit| super::lit_label(&lit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn emit_expr_stub(expr: &Expr, placeholders: &mut PlaceholderMap) -> NormNode {
        match expr {
            Expr::Path(path) => {
                let name = path
                    .path
                    .segments
                    .last()
                    .map_or_else(|| "path".to_owned(), |seg| seg.ident.to_string());
                NormNode::leaf(placeholders.placeholder(&name))
            }
            Expr::Lit(lit) => NormNode::leaf(super::super::lit_label(&lit.lit)),
            other => NormNode::leaf(format!("stub:{}", std::any::type_name_of_val(other))),
        }
    }

    fn expr_macro(expr: syn::Expr) -> NormNode {
        let mut placeholders = PlaceholderMap::default();
        let syn::Expr::Macro(mac) = expr else {
            return NormNode::leaf("not-macro");
        };
        emit_macro(&mac.mac, &mut placeholders, emit_expr_stub)
    }

    #[test]
    fn different_macro_names_differ() {
        let println_node = expr_macro(parse_quote!(println!("{}", x)));
        let eprintln_node = expr_macro(parse_quote!(eprintln!("{}", x)));
        assert_eq!(println_node.label, "macro_expand:println:paren");
        assert_eq!(eprintln_node.label, "macro_expand:eprintln:paren");
        assert_ne!(println_node.label, eprintln_node.label);
    }

    #[test]
    fn renamed_idents_inside_macro_match() {
        let mut left_map = PlaceholderMap::default();
        let left = match parse_quote!(println!("{}", a)) {
            syn::Expr::Macro(mac) => emit_macro(&mac.mac, &mut left_map, emit_expr_stub),
            _ => NormNode::leaf("not-macro"),
        };
        let mut right_map = PlaceholderMap::default();
        let right = match parse_quote!(println!("{}", b)) {
            syn::Expr::Macro(mac) => emit_macro(&mac.mac, &mut right_map, emit_expr_stub),
            _ => NormNode::leaf("not-macro"),
        };
        assert_eq!(left.label, "macro_expand:println:paren");
        assert_eq!(left, right);
    }

    #[test]
    fn delimiter_distinguishes_vec_from_println() {
        let println_node = expr_macro(parse_quote!(println!("{}", x)));
        let vec_node = expr_macro(parse_quote!(vec![1, 2]));
        assert_eq!(vec_node.label, "macro_expand:vec:bracket");
        assert_ne!(println_node.label, vec_node.label);
    }

    #[test]
    fn vec_renamed_args_share_structure() {
        let left = expr_macro(parse_quote!(vec![a, b]));
        let right = expr_macro(parse_quote!(vec![x, y]));
        assert_eq!(left.label, "macro_expand:vec:bracket");
        assert_eq!(left, right);
    }

    #[test]
    fn assert_eq_expands_to_structural_children() {
        let node = expr_macro(parse_quote!(assert_eq!(1, 2)));
        assert_eq!(node.label, "macro_expand:assert_eq:paren");
        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0].label, "lit_int");
        assert_eq!(node.children[1].label, "lit_int");
    }

    #[test]
    fn assert_and_assert_eq_differ() {
        let mut placeholders = PlaceholderMap::default();
        let left = match parse_quote!(assert!(true);) {
            syn::Stmt::Macro(mac) => emit_macro(&mac.mac, &mut placeholders, emit_expr_stub),
            _ => NormNode::leaf("not-macro"),
        };
        let mut other = PlaceholderMap::default();
        let right = match parse_quote!(assert_eq!(1, 2);) {
            syn::Stmt::Macro(mac) => emit_macro(&mac.mac, &mut other, emit_expr_stub),
            _ => NormNode::leaf("not-macro"),
        };
        assert_eq!(left.label, "macro_expand:assert:paren");
        assert_eq!(right.label, "macro_expand:assert_eq:paren");
        assert_ne!(left, right);
    }

    #[test]
    fn non_allowlisted_macro_keeps_token_tree_shape() {
        let node = expr_macro(parse_quote!(tracing::info!("hi")));
        assert_eq!(node.label, "macro:info:paren");
        assert!(node.children.iter().any(|c| c.label == "lit_str"));
    }

    #[test]
    fn empty_expand_is_leaf_and_parse_none() {
        use std::str::FromStr;

        let empty = expr_macro(parse_quote!(vec![]));
        assert_eq!(empty.label, "macro_expand:vec:bracket");
        assert!(empty.children.is_empty());
        #[expect(clippy::expect_used, reason = "test setup")]
        let bad = TokenStream::from_str("struct S;").expect("tokens");
        assert!(parse_macro_exprs(bad).is_none());
        assert_eq!(
            parse_macro_exprs(TokenStream::new()).map(|v| v.len()),
            Some(0)
        );
    }

    #[test]
    #[expect(
        clippy::cognitive_complexity,
        reason = "token-tree corpus covers delimiter and literal arms"
    )]
    fn token_trees_cover_groups_lits_and_delims() {
        use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream, TokenTree};

        let brace = expr_macro(parse_quote!(todo! { 1, 2 }));
        assert_eq!(brace.label, "macro:todo:brace");
        let empty = expr_macro(parse_quote!(todo!()));
        assert_eq!(empty.label, "macro:todo:paren");
        assert!(empty.children.is_empty());

        let rich = expr_macro(parse_quote!(m!(
            (),
            { x },
            [1],
            "s",
            b"b",
            c"c",
            b'z',
            'q',
            7,
            1.25
        )));
        assert!(rich.children.iter().any(|c| c.label == "tt_paren"));
        assert!(rich.children.iter().any(|c| c.label == "tt_brace"));
        assert!(rich.children.iter().any(|c| c.label == "tt_bracket"));
        assert!(rich.children.iter().any(|c| c.label == "lit_str"));
        assert!(rich.children.iter().any(|c| c.label == "lit_bytestr"));
        assert!(rich.children.iter().any(|c| c.label == "lit_cstr"));
        assert!(rich.children.iter().any(|c| c.label == "lit_byte"));
        assert!(rich.children.iter().any(|c| c.label == "lit_char"));
        assert!(rich.children.iter().any(|c| c.label == "lit_int"));
        assert!(rich.children.iter().any(|c| c.label == "lit_float"));

        let mut placeholders = PlaceholderMap::default();
        let mut mac: Macro = parse_quote!(m!());
        let none_stream = TokenStream::from(TokenTree::Ident(Ident::new("y", Span::call_site())));
        mac.tokens = TokenStream::from(TokenTree::Group(Group::new(Delimiter::None, none_stream)));
        let none_node = emit_macro(&mac, &mut placeholders, emit_expr_stub);
        assert!(none_node.children.iter().any(|c| c.label == "tt_none"));
        assert_eq!(label_from_lit_stream(TokenStream::new()), "lit_other");
        assert_eq!(
            super::super::lit_label(&syn::Lit::Verbatim(
                proc_macro2::Literal::i32_unsuffixed(0,)
            )),
            "lit_other"
        );
    }
}
