//! Macro emission: name, delimiter, and `TokenTree` structure.

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::{Macro, MacroDelimiter};

use crate::normalize::placeholders::PlaceholderMap;
use crate::normalize::tree::NormNode;

/// Emits a normalized tree for a macro invocation.
#[must_use]
pub fn emit_macro(mac: &Macro, placeholders: &mut PlaceholderMap) -> NormNode {
    let name = macro_name(mac);
    let delim = delimiter_label(&mac.delimiter);
    let label = format!("macro:{name}:{delim}");
    let children = emit_token_stream(mac.tokens.clone(), placeholders);
    if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    }
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
    syn::parse2::<syn::Lit>(stream).map_or("lit_other", |lit| {
        lit_text_label(&lit).unwrap_or_else(|| lit_numeric_label(&lit))
    })
}

const fn lit_text_label(lit: &syn::Lit) -> Option<&'static str> {
    match lit {
        syn::Lit::Str(_) => Some("lit_str"),
        syn::Lit::ByteStr(_) => Some("lit_bytestr"),
        syn::Lit::CStr(_) => Some("lit_cstr"),
        _ => lit_byte_char_label(lit),
    }
}

const fn lit_byte_char_label(lit: &syn::Lit) -> Option<&'static str> {
    match lit {
        syn::Lit::Byte(_) => Some("lit_byte"),
        syn::Lit::Char(_) => Some("lit_char"),
        _ => None,
    }
}

const fn lit_numeric_label(lit: &syn::Lit) -> &'static str {
    match lit {
        syn::Lit::Int(_) => "lit_int",
        syn::Lit::Float(_) => "lit_float",
        // `true`/`false` are idents in token trees, not literals.
        _ => "lit_other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn expr_macro(expr: syn::Expr) -> NormNode {
        let mut placeholders = PlaceholderMap::default();
        let syn::Expr::Macro(mac) = expr else {
            return NormNode::leaf("not-macro");
        };
        emit_macro(&mac.mac, &mut placeholders)
    }

    #[test]
    fn different_macro_names_differ() {
        let println_node = expr_macro(parse_quote!(println!("{}", x)));
        let eprintln_node = expr_macro(parse_quote!(eprintln!("{}", x)));
        assert_eq!(println_node.label, "macro:println:paren");
        assert_eq!(eprintln_node.label, "macro:eprintln:paren");
        assert_ne!(println_node.label, eprintln_node.label);
    }

    #[test]
    fn renamed_idents_inside_macro_match() {
        let mut left_map = PlaceholderMap::default();
        let left = match parse_quote!(println!("{}", a)) {
            syn::Expr::Macro(mac) => emit_macro(&mac.mac, &mut left_map),
            _ => NormNode::leaf("not-macro"),
        };
        let mut right_map = PlaceholderMap::default();
        let right = match parse_quote!(println!("{}", b)) {
            syn::Expr::Macro(mac) => emit_macro(&mac.mac, &mut right_map),
            _ => NormNode::leaf("not-macro"),
        };
        assert_eq!(left.label, "macro:println:paren");
        assert_eq!(left, right);
    }

    #[test]
    fn delimiter_distinguishes_vec_from_println() {
        let println_node = expr_macro(parse_quote!(println!("{}", x)));
        let vec_node = expr_macro(parse_quote!(vec![1, 2]));
        assert_eq!(vec_node.label, "macro:vec:bracket");
        assert_ne!(println_node.label, vec_node.label);
    }

    #[test]
    fn assert_and_assert_eq_differ() {
        let mut placeholders = PlaceholderMap::default();
        let left = match parse_quote!(assert!(true);) {
            syn::Stmt::Macro(mac) => emit_macro(&mac.mac, &mut placeholders),
            _ => NormNode::leaf("not-macro"),
        };
        let mut other = PlaceholderMap::default();
        let right = match parse_quote!(assert_eq!(1, 2);) {
            syn::Stmt::Macro(mac) => emit_macro(&mac.mac, &mut other),
            _ => NormNode::leaf("not-macro"),
        };
        assert_eq!(left.label, "macro:assert:paren");
        assert_eq!(right.label, "macro:assert_eq:paren");
        assert_ne!(left, right);
    }

    #[test]
    fn multi_segment_path_uses_last_segment() {
        let node = expr_macro(parse_quote!(tracing::info!("hi")));
        assert_eq!(node.label, "macro:info:paren");
    }

    #[test]
    #[expect(
        clippy::cognitive_complexity,
        reason = "token-tree corpus covers delimiter and literal arms"
    )]
    fn token_trees_cover_groups_lits_and_delims() {
        use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream, TokenTree};

        let brace = expr_macro(parse_quote!(vec! { 1, 2 }));
        assert_eq!(brace.label, "macro:vec:brace");
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
        let none_node = emit_macro(&mac, &mut placeholders);
        assert!(none_node.children.iter().any(|c| c.label == "tt_none"));
        assert_eq!(label_from_lit_stream(TokenStream::new()), "lit_other");
        assert_eq!(
            lit_numeric_label(&syn::Lit::Verbatim(proc_macro2::Literal::i32_unsuffixed(0))),
            "lit_other"
        );
    }
}
