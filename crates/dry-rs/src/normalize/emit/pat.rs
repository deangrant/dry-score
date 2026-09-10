//! Pattern emission for structural fingerprints.

use syn::{Member, Pat, Path, RangeLimits};

use super::emit_block;
use super::emit_expr;
use super::mac::emit_macro;
use crate::normalize::placeholders::PlaceholderMap;
use crate::normalize::tree::NormNode;

/// Emits a normalized tree for a pattern.
pub fn emit_pat(pat: &Pat, placeholders: &mut PlaceholderMap) -> NormNode {
    match pat {
        Pat::Ident(ident) => emit_ident_pat(ident, placeholders),
        Pat::Type(ty) => NormNode::branch("pat_type", vec![emit_pat(&ty.pat, placeholders)]),
        Pat::Wild(_) => NormNode::leaf("pat_wild"),
        Pat::Lit(lit) => NormNode::leaf(lit_kind(&lit.lit)),
        Pat::Path(path) => emit_path_pat(path, placeholders),
        Pat::Rest(_) => NormNode::leaf("pat_rest"),
        Pat::Paren(paren) => emit_pat(&paren.pat, placeholders),
        Pat::Reference(reference) => {
            NormNode::branch("pat_ref", vec![emit_pat(&reference.pat, placeholders)])
        }
        Pat::Or(or_pat) => {
            let children = or_pat.cases.iter().map(|case| emit_pat(case, placeholders)).collect();
            NormNode::branch("pat_or", children)
        }
        Pat::Range(range) => emit_range_pat(range, placeholders),
        Pat::Slice(slice) => {
            let children = slice.elems.iter().map(|elem| emit_pat(elem, placeholders)).collect();
            NormNode::branch("pat_slice", children)
        }
        Pat::Tuple(tuple) => {
            let children = tuple.elems.iter().map(|elem| emit_pat(elem, placeholders)).collect();
            NormNode::branch("pat_tuple", children)
        }
        Pat::Struct(strct) => emit_struct_pat(strct, placeholders),
        Pat::TupleStruct(ts) => emit_tuple_struct_pat(ts, placeholders),
        Pat::Macro(mac) => emit_macro(&mac.mac, placeholders),
        Pat::Const(expr_const) => NormNode::branch(
            "pat_const",
            vec![emit_block(&expr_const.block, placeholders)],
        ),
        _ => NormNode::leaf("pat_other"),
    }
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
    let label = path
        .segments
        .iter()
        .map(|seg| placeholders.placeholder(&seg.ident.to_string()))
        .collect::<Vec<_>>()
        .join("::");
    NormNode::leaf(format!("path:{label}"))
}

fn emit_range_pat(range: &syn::ExprRange, placeholders: &mut PlaceholderMap) -> NormNode {
    let label = match range.limits {
        RangeLimits::HalfOpen(_) => "pat_range",
        RangeLimits::Closed(_) => "pat_range_inclusive",
    };
    let mut children = Vec::new();
    if let Some(start) = &range.start {
        children.push(emit_expr(start, placeholders));
    }
    if let Some(end) = &range.end {
        children.push(emit_expr(end, placeholders));
    }
    if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    }
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
    match lit {
        syn::Lit::Str(_) => "lit_str",
        syn::Lit::ByteStr(_) => "lit_bytestr",
        syn::Lit::CStr(_) => "lit_cstr",
        syn::Lit::Byte(_) => "lit_byte",
        syn::Lit::Char(_) => "lit_char",
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
