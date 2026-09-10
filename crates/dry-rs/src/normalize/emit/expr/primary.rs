//! Primary expression emitters.

use syn::Lit;

use super::super::emit_pat;
use super::super::ops::{bin_op_label, un_op_label};
use super::emit_expr;
use crate::normalize::placeholders::PlaceholderMap;
use crate::normalize::tree::NormNode;

pub(super) fn emit_binary(bin: &syn::ExprBinary, placeholders: &mut PlaceholderMap) -> NormNode {
    NormNode::branch(
        format!("binary:{}", bin_op_label(&bin.op)),
        vec![
            emit_expr(&bin.left, placeholders),
            emit_expr(&bin.right, placeholders),
        ],
    )
}

pub(super) fn emit_unary(unary: &syn::ExprUnary, placeholders: &mut PlaceholderMap) -> NormNode {
    NormNode::branch(
        format!("unary:{}", un_op_label(&unary.op)),
        vec![emit_expr(&unary.expr, placeholders)],
    )
}

pub(super) fn emit_return(ret: &syn::ExprReturn, placeholders: &mut PlaceholderMap) -> NormNode {
    ret.expr.as_ref().map_or_else(
        || NormNode::leaf("return"),
        |inner| NormNode::branch("return", vec![emit_expr(inner, placeholders)]),
    )
}

pub(super) fn emit_list(
    label: &str,
    elems: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    let children = elems.iter().map(|elem| emit_expr(elem, placeholders)).collect();
    NormNode::branch(label, children)
}

pub(super) fn emit_path(path: &syn::ExprPath, placeholders: &mut PlaceholderMap) -> NormNode {
    if let Some(ident) = path.path.get_ident() {
        return NormNode::leaf(placeholders.placeholder(&ident.to_string()));
    }
    let label = path
        .path
        .segments
        .iter()
        .map(|seg| placeholders.placeholder(&seg.ident.to_string()))
        .collect::<Vec<_>>()
        .join("::");
    NormNode::leaf(format!("path:{label}"))
}

pub(super) fn emit_call(call: &syn::ExprCall, placeholders: &mut PlaceholderMap) -> NormNode {
    // dry-rs:ignore. Call vs method share arg folding by design.
    let mut children = vec![emit_expr(&call.func, placeholders)];
    append_args(&mut children, &call.args, placeholders);
    NormNode::branch("call", children)
}

pub(super) fn emit_method_call(
    method: &syn::ExprMethodCall,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    // dry-rs:ignore. Call vs method share arg folding by design.
    let mut children = vec![
        emit_expr(&method.receiver, placeholders),
        NormNode::leaf(placeholders.placeholder(&method.method.to_string())),
    ];
    append_args(&mut children, &method.args, placeholders);
    NormNode::branch("method", children)
}

fn append_args(
    children: &mut Vec<NormNode>,
    args: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
    placeholders: &mut PlaceholderMap,
) {
    children.extend(args.iter().map(|arg| emit_expr(arg, placeholders)));
}

pub(super) fn emit_field(field: &syn::ExprField, placeholders: &mut PlaceholderMap) -> NormNode {
    NormNode::branch(
        "field",
        vec![
            emit_expr(&field.base, placeholders),
            NormNode::leaf(placeholders.placeholder(&field_member_name(&field.member))),
        ],
    )
}

pub(super) fn emit_closure(
    closure: &syn::ExprClosure,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    let mut children: Vec<NormNode> =
        closure.inputs.iter().map(|pat| emit_pat(pat, placeholders)).collect();
    children.push(emit_expr(&closure.body, placeholders));
    NormNode::branch("closure", children)
}

fn field_member_name(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(ident) => ident.to_string(),
        syn::Member::Unnamed(index) => index.index.to_string(),
    }
}

pub(super) const fn lit_label(lit: &Lit) -> &'static str {
    match lit_text_label(lit) {
        Some(label) => label,
        None => lit_numeric_label(lit),
    }
}

const fn lit_text_label(lit: &Lit) -> Option<&'static str> {
    match lit {
        Lit::Str(_) => Some("lit_str"),
        Lit::ByteStr(_) => Some("lit_bytestr"),
        Lit::CStr(_) => Some("lit_cstr"),
        _ => lit_byte_char_label(lit),
    }
}

const fn lit_byte_char_label(lit: &Lit) -> Option<&'static str> {
    match lit {
        Lit::Byte(_) => Some("lit_byte"),
        Lit::Char(_) => Some("lit_char"),
        _ => None,
    }
}

const fn lit_numeric_label(lit: &Lit) -> &'static str {
    match lit {
        Lit::Int(_) => "lit_int",
        Lit::Float(_) => "lit_float",
        Lit::Bool(_) => "lit_bool",
        _ => "lit_other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::placeholders::PlaceholderMap;
    use syn::parse_quote;

    #[test]
    #[expect(
        clippy::cognitive_complexity,
        reason = "literal label corpus is intentionally flat assertions"
    )]
    fn literal_labels() {
        assert_eq!(lit_label(&parse_quote!("hi")), "lit_str");
        assert_eq!(lit_label(&parse_quote!(b"hi")), "lit_bytestr");
        assert_eq!(lit_label(&parse_quote!(b'x')), "lit_byte");
        assert_eq!(lit_label(&parse_quote!('x')), "lit_char");
        assert_eq!(lit_label(&parse_quote!(1)), "lit_int");
        assert_eq!(lit_label(&parse_quote!(1.5)), "lit_float");
        assert_eq!(lit_label(&parse_quote!(true)), "lit_bool");
        assert_eq!(lit_label(&parse_quote!(c"hi")), "lit_cstr");
        assert_eq!(
            lit_label(&syn::Lit::Verbatim(proc_macro2::Literal::i32_unsuffixed(0))),
            "lit_other"
        );
    }

    #[test]
    fn expression_helpers() {
        let mut p = PlaceholderMap::default();
        let _ = emit_binary(&parse_quote!(a + b), &mut p);
        let _ = emit_unary(&parse_quote!(-a), &mut p);
        let _ = emit_return(&parse_quote!(return 1), &mut p);
        let _ = emit_return(&parse_quote!(return), &mut p);
        let call: syn::ExprCall = parse_quote!(f(1, 2));
        let _ = emit_call(&call, &mut p);
        let method: syn::ExprMethodCall = parse_quote!(a.foo(1));
        let _ = emit_method_call(&method, &mut p);
        let field: syn::ExprField = parse_quote!(a.b);
        let _ = emit_field(&field, &mut p);
        let field0: syn::ExprField = parse_quote!(a.0);
        let _ = emit_field(&field0, &mut p);
        let path: syn::ExprPath = parse_quote!(a::b);
        let _ = emit_path(&path, &mut p);
        let path_id: syn::ExprPath = parse_quote!(x);
        let _ = emit_path(&path_id, &mut p);
        let closure: syn::ExprClosure = parse_quote!(|x| x);
        let _ = emit_closure(&closure, &mut p);
        let elems: syn::ExprArray = parse_quote!([1, 2]);
        let _ = emit_list("array", &elems.elems, &mut p);
    }
}
