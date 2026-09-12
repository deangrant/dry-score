//! Primary expression emitters.

use super::super::emit_pat;
use super::super::ops::{bin_op_label, un_op_label};
use super::super::shared::{emit_path_segments, member_name};
use super::emit_expr;
use super::wrap::emit_optional_inner;
use crate::normalize::placeholders::PlaceholderMap;
use dry_core::NormNode;

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
    // dry-rs:ignore. Thin optional-inner wrappers share shape by design.
    emit_optional_inner("return", ret.expr.as_deref(), placeholders)
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
    emit_path_segments(&path.path, placeholders)
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
            NormNode::leaf(placeholders.placeholder(&member_name(&field.member))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::placeholders::PlaceholderMap;
    use syn::parse_quote;

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
