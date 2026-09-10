//! Pattern emission for structural fingerprints.

use syn::{Member, Pat, Path, RangeLimits};

use super::emit_block;
use super::emit_expr;
use super::mac::emit_macro;
use crate::normalize::placeholders::PlaceholderMap;
use crate::normalize::tree::NormNode;

/// Emits a normalized tree for a pattern.
pub fn emit_pat(pat: &Pat, placeholders: &mut PlaceholderMap) -> NormNode {
    try_emit_simple(pat, placeholders)
        .or_else(|| try_emit_wrap_pat(pat, placeholders))
        .or_else(|| try_emit_compound(pat, placeholders))
        .or_else(|| try_emit_pathish(pat, placeholders))
        .unwrap_or_else(|| NormNode::leaf("pat_other"))
}

fn try_emit_simple(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    try_emit_binding(pat, placeholders).or_else(|| try_emit_atom_pat(pat))
}

fn try_emit_binding(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    match pat {
        Pat::Ident(ident) => Some(emit_ident_pat(ident, placeholders)),
        Pat::Type(ty) => Some(NormNode::branch(
            "pat_type",
            vec![emit_pat(&ty.pat, placeholders)],
        )),
        _ => None,
    }
}

fn try_emit_atom_pat(pat: &Pat) -> Option<NormNode> {
    match pat {
        Pat::Wild(_) => Some(NormNode::leaf("pat_wild")),
        Pat::Lit(lit) => Some(NormNode::leaf(lit_kind(&lit.lit))),
        Pat::Rest(_) => Some(NormNode::leaf("pat_rest")),
        _ => None,
    }
}

fn try_emit_wrap_pat(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    match pat {
        Pat::Paren(paren) => Some(emit_pat(&paren.pat, placeholders)),
        Pat::Reference(reference) => Some(NormNode::branch(
            "pat_ref",
            vec![emit_pat(&reference.pat, placeholders)],
        )),
        _ => None,
    }
}

fn try_emit_compound(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    try_emit_or_range(pat, placeholders).or_else(|| try_emit_seq_pat(pat, placeholders))
}

fn try_emit_or_range(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    match pat {
        Pat::Or(or_pat) => {
            let children = or_pat.cases.iter().map(|case| emit_pat(case, placeholders)).collect();
            Some(NormNode::branch("pat_or", children))
        }
        Pat::Range(range) => Some(emit_range_pat(range, placeholders)),
        _ => None,
    }
}

fn try_emit_seq_pat(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    match pat {
        Pat::Slice(slice) => {
            let children = slice.elems.iter().map(|elem| emit_pat(elem, placeholders)).collect();
            Some(NormNode::branch("pat_slice", children))
        }
        Pat::Tuple(tuple) => {
            let children = tuple.elems.iter().map(|elem| emit_pat(elem, placeholders)).collect();
            Some(NormNode::branch("pat_tuple", children))
        }
        _ => None,
    }
}

fn try_emit_pathish(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    try_emit_named_path(pat, placeholders).or_else(|| try_emit_pat_macro_const(pat, placeholders))
}

fn try_emit_named_path(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match pat {
        Pat::Path(path) => emit_path_pat(path, placeholders),
        Pat::Struct(strct) => emit_struct_pat(strct, placeholders),
        Pat::TupleStruct(ts) => emit_tuple_struct_pat(ts, placeholders),
        _ => return None,
    })
}

fn try_emit_pat_macro_const(pat: &Pat, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match pat {
        Pat::Macro(mac) => emit_macro(&mac.mac, placeholders),
        Pat::Const(expr_const) => NormNode::branch(
            "pat_const",
            vec![emit_block(&expr_const.block, placeholders)],
        ),
        _ => return None,
    })
}

fn emit_ident_pat(ident: &syn::PatIdent, placeholders: &mut PlaceholderMap) -> NormNode {
    let binding = NormNode::leaf(placeholders.placeholder(&ident.ident.to_string()));
    match &ident.subpat {
        Some((_, subpat)) => {
            NormNode::branch("pat_ident", vec![binding, emit_pat(subpat, placeholders)])
        }
        None => binding,
    }
}

fn emit_path_pat(path: &syn::ExprPath, placeholders: &mut PlaceholderMap) -> NormNode {
    emit_path_segments(&path.path, placeholders)
}

fn emit_path_segments(path: &Path, placeholders: &mut PlaceholderMap) -> NormNode {
    if let Some(ident) = path.get_ident() {
        return NormNode::leaf(placeholders.placeholder(&ident.to_string()));
    }
    emit_multi_segment_path(path, placeholders)
}

fn emit_multi_segment_path(path: &Path, placeholders: &mut PlaceholderMap) -> NormNode {
    let label = path
        .segments
        .iter()
        .map(|seg| placeholders.placeholder(&seg.ident.to_string()))
        .collect::<Vec<_>>()
        .join("::");
    NormNode::leaf(format!("path:{label}"))
}

fn emit_range_pat(range: &syn::ExprRange, placeholders: &mut PlaceholderMap) -> NormNode {
    let label = range_pat_label(range.limits);
    let children = range_bound_nodes(range, placeholders);
    if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    }
}

const fn range_pat_label(limits: RangeLimits) -> &'static str {
    match limits {
        RangeLimits::HalfOpen(_) => "pat_range",
        RangeLimits::Closed(_) => "pat_range_inclusive",
    }
}

fn range_bound_nodes(range: &syn::ExprRange, placeholders: &mut PlaceholderMap) -> Vec<NormNode> {
    let mut children = Vec::new();
    if let Some(start) = &range.start {
        children.push(emit_expr(start, placeholders));
    }
    if let Some(end) = &range.end {
        children.push(emit_expr(end, placeholders));
    }
    children
}

fn emit_struct_pat(strct: &syn::PatStruct, placeholders: &mut PlaceholderMap) -> NormNode {
    let mut children = path_segment_nodes(&strct.path, placeholders);
    for field in &strct.fields {
        children.push(NormNode::leaf(
            placeholders.placeholder(&member_name(&field.member)),
        ));
        children.push(emit_pat(&field.pat, placeholders));
    }
    if strct.rest.is_some() {
        children.push(NormNode::leaf("pat_rest"));
    }
    NormNode::branch("pat_struct", children)
}

fn emit_tuple_struct_pat(
    tuple_struct: &syn::PatTupleStruct,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    let mut children = path_segment_nodes(&tuple_struct.path, placeholders);
    children.extend(tuple_struct.elems.iter().map(|elem| emit_pat(elem, placeholders)));
    NormNode::branch("pat_tuple_struct", children)
}

fn path_segment_nodes(path: &Path, placeholders: &mut PlaceholderMap) -> Vec<NormNode> {
    path.segments
        .iter()
        .map(|seg| NormNode::leaf(placeholders.placeholder(&seg.ident.to_string())))
        .collect()
}

fn member_name(member: &Member) -> String {
    match member {
        Member::Named(ident) => ident.to_string(),
        Member::Unnamed(index) => index.index.to_string(),
    }
}

const fn lit_kind(lit: &syn::Lit) -> &'static str {
    match lit_text_kind(lit) {
        Some(label) => label,
        None => lit_numeric_kind(lit),
    }
}

const fn lit_text_kind(lit: &syn::Lit) -> Option<&'static str> {
    match lit {
        syn::Lit::Str(_) => Some("lit_str"),
        syn::Lit::ByteStr(_) => Some("lit_bytestr"),
        syn::Lit::CStr(_) => Some("lit_cstr"),
        _ => lit_byte_char_kind(lit),
    }
}

const fn lit_byte_char_kind(lit: &syn::Lit) -> Option<&'static str> {
    match lit {
        syn::Lit::Byte(_) => Some("lit_byte"),
        syn::Lit::Char(_) => Some("lit_char"),
        _ => None,
    }
}

const fn lit_numeric_kind(lit: &syn::Lit) -> &'static str {
    match lit {
        syn::Lit::Int(_) => "lit_int",
        syn::Lit::Float(_) => "lit_float",
        syn::Lit::Bool(_) => "lit_bool",
        _ => "lit_other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    #[expect(
        clippy::cognitive_complexity,
        reason = "pattern label corpus is intentionally flat assertions"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "pattern label corpus is intentionally flat assertions"
    )]
    fn pattern_labels() {
        let mut placeholders = PlaceholderMap::default();
        assert_eq!(
            emit_pat(&parse_quote!(_), &mut placeholders).label,
            "pat_wild"
        );
        assert_eq!(
            emit_pat(&parse_quote!(1..=2), &mut placeholders).label,
            "pat_range_inclusive"
        );
        assert_eq!(
            emit_pat(&parse_quote!(1..2), &mut placeholders).label,
            "pat_range"
        );
        assert_eq!(
            emit_pat(&parse_quote!(A | B), &mut placeholders).label,
            "pat_or"
        );
        assert_eq!(
            emit_pat(&parse_quote!(&x), &mut placeholders).label,
            "pat_ref"
        );
        assert_eq!(
            emit_pat(&parse_quote!([a, .., b]), &mut placeholders).label,
            "pat_slice"
        );
        assert_eq!(
            emit_pat(&parse_quote!(0), &mut placeholders).label,
            "lit_int"
        );
        let path_pat = emit_pat(
            &parse_quote!(std::option::Option::None),
            &mut PlaceholderMap::default(),
        );
        assert!(path_pat.label.starts_with("path:"));
        let at_pat = emit_pat(&parse_quote!(x @ Some(_)), &mut PlaceholderMap::default());
        assert_eq!(at_pat.label, "pat_ident");
        let mac = emit_pat(&parse_quote!(stringify!(x)), &mut PlaceholderMap::default());
        assert!(mac.label.starts_with("macro:"));
        assert_eq!(
            emit_pat(&parse_quote!(true), &mut PlaceholderMap::default()).label,
            "lit_bool"
        );
        assert_eq!(
            emit_pat(&parse_quote!("hi"), &mut PlaceholderMap::default()).label,
            "lit_str"
        );
        assert_eq!(
            emit_pat(&parse_quote!(b"hi"), &mut PlaceholderMap::default()).label,
            "lit_bytestr"
        );
        assert_eq!(
            emit_pat(&parse_quote!(c"hi"), &mut PlaceholderMap::default()).label,
            "lit_cstr"
        );
        assert_eq!(
            emit_pat(&parse_quote!(b'x'), &mut PlaceholderMap::default()).label,
            "lit_byte"
        );
        assert_eq!(
            emit_pat(&parse_quote!('x'), &mut PlaceholderMap::default()).label,
            "lit_char"
        );
        assert_eq!(
            emit_pat(&parse_quote!(1.5), &mut PlaceholderMap::default()).label,
            "lit_float"
        );
        assert_eq!(
            emit_pat(&parse_quote!((x)), &mut PlaceholderMap::default()).label,
            emit_pat(&parse_quote!(x), &mut PlaceholderMap::default()).label
        );
        assert_eq!(
            emit_pat(&parse_quote!((a, b)), &mut PlaceholderMap::default()).label,
            "pat_tuple"
        );
        let type_pat = match parse_quote!(let x: i32 = 1;) {
            syn::Stmt::Local(local) => emit_pat(&local.pat, &mut PlaceholderMap::default()),
            _ => NormNode::leaf("not-local"),
        };
        assert_eq!(type_pat.label, "pat_type");
        let open_full = Pat::Range(syn::ExprRange {
            attrs: Vec::new(),
            start: None,
            limits: RangeLimits::HalfOpen(syn::token::DotDot::default()),
            end: None,
        });
        assert_eq!(
            emit_pat(&open_full, &mut PlaceholderMap::default()).label,
            "pat_range"
        );
        let closed_full = Pat::Range(syn::ExprRange {
            attrs: Vec::new(),
            start: None,
            limits: RangeLimits::Closed(syn::token::DotDotEq::default()),
            end: None,
        });
        assert_eq!(
            emit_pat(&closed_full, &mut PlaceholderMap::default()).label,
            "pat_range_inclusive"
        );
        let const_pat = emit_pat(&parse_quote!(const { 1 }), &mut PlaceholderMap::default());
        assert_eq!(const_pat.label, "pat_const");
        let unnamed = emit_pat(
            &parse_quote!(Pair { 0: a, 1: b }),
            &mut PlaceholderMap::default(),
        );
        assert_eq!(unnamed.label, "pat_struct");
        assert_eq!(
            emit_pat(&parse_quote!(Point(a, b)), &mut PlaceholderMap::default()).label,
            "pat_tuple_struct"
        );
        let single = emit_pat(&parse_quote!(None), &mut PlaceholderMap::default());
        assert!(!single.label.starts_with("path:"));
        let path_ident = emit_path_segments(&parse_quote!(Foo), &mut PlaceholderMap::default());
        assert!(!path_ident.label.starts_with("path:"));
        assert!(
            try_emit_pat_macro_const(&parse_quote!(_), &mut PlaceholderMap::default()).is_none()
        );
        assert_eq!(
            lit_kind(&syn::Lit::Verbatim(proc_macro2::Literal::i32_unsuffixed(0))),
            "lit_other"
        );
    }

    #[test]
    fn renamed_bindings_in_slice_match() {
        let left = emit_pat(&parse_quote!([a, .., b]), &mut PlaceholderMap::default());
        let right = emit_pat(&parse_quote!([x, .., y]), &mut PlaceholderMap::default());
        assert_eq!(left, right);
    }

    #[test]
    fn struct_pat_includes_path() {
        let node = emit_pat(&parse_quote!(Foo { y, .. }), &mut PlaceholderMap::default());
        assert_eq!(node.label, "pat_struct");
        assert!(node.children.iter().any(|c| c.label == "pat_rest"));
        assert!(node.children.len() >= 3);
    }
}
