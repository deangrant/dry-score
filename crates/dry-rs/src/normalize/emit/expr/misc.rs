//! Misc expression emitters for previously uncovered variants.

use super::super::shared::{member_name, path_segment_leaves};
use super::emit_expr;
use super::wrap::{
    emit_labeled_block, emit_optional_inner, emit_pair, emit_range_like, emit_unary_wrap,
};
use crate::normalize::placeholders::PlaceholderMap;
use crate::normalize::tree::NormNode;

pub(super) fn emit_break(
    expr_break: &syn::ExprBreak,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    // dry-rs:ignore. Thin optional-inner wrappers share shape by design.
    emit_optional_inner("break", expr_break.expr.as_deref(), placeholders)
}

pub(super) fn emit_continue() -> NormNode {
    NormNode::leaf("continue")
}

pub(super) fn emit_cast(cast: &syn::ExprCast, placeholders: &mut PlaceholderMap) -> NormNode {
    emit_unary_wrap("cast", &cast.expr, placeholders)
}

pub(super) fn emit_range(range: &syn::ExprRange, placeholders: &mut PlaceholderMap) -> NormNode {
    emit_range_like(range, "range", "range_inclusive", placeholders)
}

pub(super) fn emit_struct(
    expr_struct: &syn::ExprStruct,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    let mut children = path_segment_leaves(&expr_struct.path, placeholders);
    for field in &expr_struct.fields {
        children.push(NormNode::leaf(
            placeholders.placeholder(&member_name(&field.member)),
        ));
        children.push(emit_expr(&field.expr, placeholders));
    }
    if let Some(rest) = &expr_struct.rest {
        children.push(emit_expr(rest, placeholders));
    }
    NormNode::branch("struct", children)
}

pub(super) fn emit_repeat(repeat: &syn::ExprRepeat, placeholders: &mut PlaceholderMap) -> NormNode {
    // dry-rs:ignore. Thin emit_pair wrappers share shape by design.
    emit_pair("repeat", &repeat.expr, &repeat.len, placeholders)
}

pub(super) fn emit_unsafe(
    expr_unsafe: &syn::ExprUnsafe,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    emit_labeled_block("unsafe", &expr_unsafe.block, placeholders)
}

pub(super) fn emit_let(expr_let: &syn::ExprLet, placeholders: &mut PlaceholderMap) -> NormNode {
    NormNode::branch(
        "let",
        vec![
            super::super::emit_pat(&expr_let.pat, placeholders),
            emit_expr(&expr_let.expr, placeholders),
        ],
    )
}

pub(super) fn emit_try_block(
    try_block: &syn::ExprTryBlock,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    emit_labeled_block("try_block", &try_block.block, placeholders)
}

pub(super) fn emit_const_block(
    expr_const: &syn::ExprConst,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    emit_labeled_block("const_block", &expr_const.block, placeholders)
}

pub(super) fn emit_infer() -> NormNode {
    NormNode::leaf("infer")
}

pub(super) fn emit_raw_addr(raw: &syn::ExprRawAddr, placeholders: &mut PlaceholderMap) -> NormNode {
    emit_unary_wrap("raw_ref", &raw.expr, placeholders)
}

pub(super) fn emit_yield(
    expr_yield: &syn::ExprYield,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    // dry-rs:ignore. Thin optional-inner wrappers share shape by design.
    emit_optional_inner("yield", expr_yield.expr.as_deref(), placeholders)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::Expr;
    use syn::parse_quote;

    #[test]
    #[expect(
        clippy::cognitive_complexity,
        reason = "misc emitter corpus is intentionally flat assertions"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "misc emitter corpus is intentionally flat assertions"
    )]
    fn misc_emitter_labels() {
        let mut p = PlaceholderMap::default();
        assert_eq!(emit_expr(&parse_quote!(break), &mut p).label, "break");
        assert_eq!(emit_expr(&parse_quote!(break 1), &mut p).label, "break");
        assert_eq!(emit_expr(&parse_quote!(continue), &mut p).label, "continue");
        assert_eq!(emit_expr(&parse_quote!(x as u32), &mut p).label, "cast");
        assert_eq!(emit_expr(&parse_quote!(1..n), &mut p).label, "range");
        assert_eq!(
            emit_expr(&parse_quote!(1..=n), &mut p).label,
            "range_inclusive"
        );
        let open_full = Expr::Range(syn::ExprRange {
            attrs: Vec::new(),
            start: None,
            limits: syn::RangeLimits::HalfOpen(syn::token::DotDot::default()),
            end: None,
        });
        assert_eq!(emit_expr(&open_full, &mut p).label, "range");
        let closed_full = Expr::Range(syn::ExprRange {
            attrs: Vec::new(),
            start: None,
            limits: syn::RangeLimits::Closed(syn::token::DotDotEq::default()),
            end: None,
        });
        assert_eq!(emit_expr(&closed_full, &mut p).label, "range_inclusive");
        assert_eq!(
            emit_expr(&parse_quote!(Point { x: 1, y: 2, ..base }), &mut p).label,
            "struct"
        );
        assert_eq!(
            emit_expr(&parse_quote!(Pair { 0: 1, 1: 2 }), &mut p).label,
            "struct"
        );
        assert_eq!(emit_expr(&parse_quote!([0; 4]), &mut p).label, "repeat");
        assert_eq!(
            emit_expr(&parse_quote!(unsafe { 1 }), &mut p).label,
            "unsafe"
        );
        assert_eq!(
            emit_expr(&parse_quote!(const { 1 }), &mut p).label,
            "const_block"
        );
        assert_eq!(
            emit_expr(&parse_quote!(try { 1 }), &mut p).label,
            "try_block"
        );
        assert_eq!(
            emit_expr(&parse_quote!(&raw const x), &mut p).label,
            "raw_ref"
        );
        let infer = Expr::Infer(syn::ExprInfer {
            attrs: Vec::new(),
            underscore_token: syn::token::Underscore::default(),
        });
        assert_eq!(emit_expr(&infer, &mut p).label, "infer");
        let yield_expr = Expr::Yield(syn::ExprYield {
            attrs: Vec::new(),
            yield_token: syn::token::Yield::default(),
            expr: None,
        });
        assert_eq!(emit_expr(&yield_expr, &mut p).label, "yield");
        let yield_val = Expr::Yield(syn::ExprYield {
            attrs: Vec::new(),
            yield_token: syn::token::Yield::default(),
            expr: Some(Box::new(parse_quote!(1))),
        });
        assert_eq!(emit_expr(&yield_val, &mut p).label, "yield");
        let grouped = Expr::Group(syn::ExprGroup {
            attrs: Vec::new(),
            group_token: syn::token::Group {
                span: proc_macro2::Span::call_site(),
            },
            expr: Box::new(parse_quote!(1)),
        });
        assert_eq!(emit_expr(&grouped, &mut p).label, "lit_int");
        assert_eq!(emit_expr(&parse_quote!((1)), &mut p).label, "lit_int");
    }

    #[test]
    fn cast_and_break_are_location_independent() {
        let mut left_map = PlaceholderMap::default();
        let left_cast = emit_expr(&parse_quote!(x as u32), &mut left_map);
        let mut right_map = PlaceholderMap::default();
        let right_cast = emit_expr(&parse_quote!(x as i64), &mut right_map);
        assert_eq!(left_cast, right_cast);
        let mut first_map = PlaceholderMap::default();
        let first_break = emit_expr(&parse_quote!(break), &mut first_map);
        let mut second_map = PlaceholderMap::default();
        let second_break = emit_expr(&parse_quote!(break), &mut second_map);
        assert_eq!(first_break, second_break);
    }
}
