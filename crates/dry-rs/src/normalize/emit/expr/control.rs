//! Control-flow expression emitters.

use super::super::emit_pat;
use super::emit_expr;
use super::wrap::{emit_labeled_block, emit_pair, emit_unary_wrap};
use crate::normalize::placeholders::PlaceholderMap;
use crate::normalize::tree::NormNode;

pub(super) fn emit_if(expr_if: &syn::ExprIf, placeholders: &mut PlaceholderMap) -> NormNode {
    let mut children = vec![
        emit_expr(&expr_if.cond, placeholders),
        super::super::emit_block(&expr_if.then_branch, placeholders),
    ];
    if let Some((_, else_branch)) = &expr_if.else_branch {
        children.push(emit_expr(else_branch, placeholders));
    }
    NormNode::branch("if", children)
}

pub(super) fn emit_while(
    expr_while: &syn::ExprWhile,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    NormNode::branch(
        "while",
        vec![
            emit_expr(&expr_while.cond, placeholders),
            super::super::emit_block(&expr_while.body, placeholders),
        ],
    )
}

pub(super) fn emit_for(for_loop: &syn::ExprForLoop, placeholders: &mut PlaceholderMap) -> NormNode {
    NormNode::branch(
        "for",
        vec![
            emit_pat(&for_loop.pat, placeholders),
            emit_expr(&for_loop.expr, placeholders),
            super::super::emit_block(&for_loop.body, placeholders),
        ],
    )
}

pub(super) fn emit_loop(expr_loop: &syn::ExprLoop, placeholders: &mut PlaceholderMap) -> NormNode {
    emit_labeled_block("loop", &expr_loop.body, placeholders)
}

pub(super) fn emit_match(
    expr_match: &syn::ExprMatch,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    let mut children = vec![emit_expr(&expr_match.expr, placeholders)];
    for arm in &expr_match.arms {
        children.push(NormNode::branch(
            "arm",
            vec![
                emit_pat(&arm.pat, placeholders),
                emit_expr(&arm.body, placeholders),
            ],
        ));
    }
    NormNode::branch("match", children)
}

pub(super) fn emit_assign(assign: &syn::ExprAssign, placeholders: &mut PlaceholderMap) -> NormNode {
    // dry-rs:ignore. Thin emit_pair wrappers share shape by design.
    emit_pair("assign", &assign.left, &assign.right, placeholders)
}

pub(super) fn emit_async(
    expr_async: &syn::ExprAsync,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    emit_labeled_block("async", &expr_async.block, placeholders)
}

pub(super) fn emit_await(
    expr_await: &syn::ExprAwait,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    emit_unary_wrap("await", &expr_await.base, placeholders)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::placeholders::PlaceholderMap;
    use syn::parse_quote;

    #[test]
    fn control_emitters() {
        let mut p = PlaceholderMap::default();
        let _ = emit_if(&parse_quote!(if true { 1 } else { 0 }), &mut p);
        let _ = emit_if(
            &parse_quote!(if true {
                1
            }),
            &mut p,
        );
        let _ = emit_while(
            &parse_quote!(while true {
                break;
            }),
            &mut p,
        );
        let _ = emit_for(
            &parse_quote!(for x in xs {
                x;
            }),
            &mut p,
        );
        let _ = emit_loop(
            &parse_quote!(loop {
                break;
            }),
            &mut p,
        );
        let _ = emit_match(
            &parse_quote!(match x {
                1 => 2,
                _ => 3,
            }),
            &mut p,
        );
        let _ = emit_assign(&parse_quote!(a = b), &mut p);
        let _ = emit_async(&parse_quote!(async { 1 }), &mut p);
        let _ = emit_await(&parse_quote!(x.await), &mut p);
    }
}
