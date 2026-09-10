//! Misc expression emitters for previously uncovered variants.

use syn::{Member, RangeLimits};

use super::super::emit_pat;
use super::emit_expr;
use crate::normalize::placeholders::PlaceholderMap;
use crate::normalize::tree::NormNode;

pub(super) fn emit_break(
    expr_break: &syn::ExprBreak,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    expr_break.expr.as_ref().map_or_else(
        || NormNode::leaf("break"),
        |inner| NormNode::branch("break", vec![emit_expr(inner, placeholders)]),
    )
}

pub(super) fn emit_continue() -> NormNode {
    NormNode::leaf("continue")
}

pub(super) fn emit_cast(cast: &syn::ExprCast, placeholders: &mut PlaceholderMap) -> NormNode {
    NormNode::branch("cast", vec![emit_expr(&cast.expr, placeholders)])
}

pub(super) fn emit_range(range: &syn::ExprRange, placeholders: &mut PlaceholderMap) -> NormNode {
    let label = range_label(range.limits);
    let children = range_bound_nodes(range, placeholders);
    if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    }
}

const fn range_label(limits: RangeLimits) -> &'static str {
    match limits {
        RangeLimits::HalfOpen(_) => "range",
        RangeLimits::Closed(_) => "range_inclusive",
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

pub(super) fn emit_struct(
    expr_struct: &syn::ExprStruct,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    let mut children = Vec::new();
    for segment in &expr_struct.path.segments {
        children.push(NormNode::leaf(
            placeholders.placeholder(&segment.ident.to_string()),
        ));
    }
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
    NormNode::branch(
        "repeat",
        vec![
            emit_expr(&repeat.expr, placeholders),
            emit_expr(&repeat.len, placeholders),
        ],
    )
}

pub(super) fn emit_unsafe(
    expr_unsafe: &syn::ExprUnsafe,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    NormNode::branch(
        "unsafe",
        vec![super::super::emit_block(&expr_unsafe.block, placeholders)],
    )
}

pub(super) fn emit_let(expr_let: &syn::ExprLet, placeholders: &mut PlaceholderMap) -> NormNode {
    NormNode::branch(
        "let",
        vec![
            emit_pat(&expr_let.pat, placeholders),
            emit_expr(&expr_let.expr, placeholders),
        ],
    )
}

pub(super) fn emit_try_block(
    try_block: &syn::ExprTryBlock,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    NormNode::branch(
        "try_block",
        vec![super::super::emit_block(&try_block.block, placeholders)],
    )
}

pub(super) fn emit_const_block(
    expr_const: &syn::ExprConst,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    NormNode::branch(
        "const_block",
        vec![super::super::emit_block(&expr_const.block, placeholders)],
    )
}

pub(super) fn emit_infer() -> NormNode {
    NormNode::leaf("infer")
}

pub(super) fn emit_raw_addr(raw: &syn::ExprRawAddr, placeholders: &mut PlaceholderMap) -> NormNode {
    NormNode::branch("raw_ref", vec![emit_expr(&raw.expr, placeholders)])
}

pub(super) fn emit_yield(
    expr_yield: &syn::ExprYield,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    expr_yield.expr.as_ref().map_or_else(
        || NormNode::leaf("yield"),
        |inner| NormNode::branch("yield", vec![emit_expr(inner, placeholders)]),
    )
}

fn member_name(member: &Member) -> String {
    match member {
        Member::Named(ident) => ident.to_string(),
        Member::Unnamed(index) => index.index.to_string(),
    }
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
            limits: RangeLimits::HalfOpen(syn::token::DotDot::default()),
            end: None,
        });
        assert_eq!(emit_expr(&open_full, &mut p).label, "range");
        let closed_full = Expr::Range(syn::ExprRange {
            attrs: Vec::new(),
            start: None,
            limits: RangeLimits::Closed(syn::token::DotDotEq::default()),
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
