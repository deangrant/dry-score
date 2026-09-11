//! Expression wrap helpers that recurse through [`emit_expr`] / [`emit_block`].

use syn::{Block, Expr, RangeLimits};

use super::super::emit_block;
use super::emit_expr;
use crate::normalize::placeholders::PlaceholderMap;
use dry_core::NormNode;

/// Emits `label` with zero or one child expression.
pub(in crate::normalize::emit) fn emit_optional_inner(
    label: &'static str,
    inner: Option<&Expr>,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    inner.map_or_else(
        || NormNode::leaf(label),
        |expr| NormNode::branch(label, vec![emit_expr(expr, placeholders)]),
    )
}

/// Emits `label` wrapping a single expression.
pub(in crate::normalize::emit) fn emit_unary_wrap(
    label: &'static str,
    expr: &Expr,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    // dry-rs:ignore. Single-child branch helpers share shape by design.
    NormNode::branch(label, vec![emit_expr(expr, placeholders)])
}

/// Emits `label` wrapping a block body.
pub(in crate::normalize::emit) fn emit_labeled_block(
    label: &'static str,
    block: &Block,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    // dry-rs:ignore. Single-child branch helpers share shape by design.
    NormNode::branch(label, vec![emit_block(block, placeholders)])
}

/// Emits a two-child branch.
pub(in crate::normalize::emit) fn emit_pair(
    label: &'static str,
    left: &Expr,
    right: &Expr,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    NormNode::branch(
        label,
        vec![
            emit_expr(left, placeholders),
            emit_expr(right, placeholders),
        ],
    )
}

/// Emits a range-like node with start/end bounds.
pub(in crate::normalize::emit) fn emit_range_like(
    range: &syn::ExprRange,
    half_open: &'static str,
    closed: &'static str,
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    let label = range_limits_label(range.limits, half_open, closed);
    let children = range_bound_nodes(range, placeholders);
    if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    }
}

const fn range_limits_label(
    limits: RangeLimits,
    half_open: &'static str,
    closed: &'static str,
) -> &'static str {
    match limits {
        RangeLimits::HalfOpen(_) => half_open,
        RangeLimits::Closed(_) => closed,
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
