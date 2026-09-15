//! CST walk helpers that emit function-like forms.

use tree_sitter::Node;

use super::names::{
    anonymous_name, function_name, method_name, named_or_anonymous, statement_body,
};
use super::{ExtractCtx, push_form, walk_children};

pub(super) fn try_walk_function(node: Node<'_>, ctx: &mut ExtractCtx<'_>) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    if node.kind() != "function_declaration" {
        return false;
    }
    maybe_emit_named_function(node, ctx);
    let name = function_name(node, ctx.source_bytes);
    walk_children(node, name.as_deref(), None, ctx);
    true
}

pub(super) fn try_walk_generator(node: Node<'_>, ctx: &mut ExtractCtx<'_>) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    if node.kind() != "generator_function_declaration" {
        return false;
    }
    maybe_emit_named_function(node, ctx);
    let name = function_name(node, ctx.source_bytes);
    walk_children(node, name.as_deref(), None, ctx);
    true
}

pub(super) fn try_walk_method(
    node: Node<'_>,
    class_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    if node.kind() != "method_definition" {
        return false;
    }
    maybe_emit_method(node, class_name, ctx);
    let name = method_name(node, class_name, ctx.source_bytes);
    walk_children(node, name.as_deref(), class_name, ctx);
    true
}

pub(super) fn try_walk_arrow(
    node: Node<'_>,
    parent_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) -> bool {
    if node.kind() != "arrow_function" {
        return false;
    }
    maybe_emit_arrow(node, parent_name, ctx);
    walk_children(node, parent_name, None, ctx);
    true
}

pub(super) fn try_walk_function_expression(
    node: Node<'_>,
    parent_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    if node.kind() != "function_expression" && node.kind() != "generator_function" {
        return false;
    }
    maybe_emit_function_expression(node, parent_name, ctx);
    let name = named_or_anonymous(node, parent_name, "$function", ctx.source_bytes);
    walk_children(node, Some(name.as_str()), None, ctx);
    true
}

fn maybe_emit_named_function(node: Node<'_>, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. CC-driven emit shells; parallel shape is intentional.
    if let Some(body) = node.child_by_field_name("body")
        && let Some(name) = function_name(node, ctx.source_bytes)
    {
        push_form(body, &name, ctx);
    }
}

fn maybe_emit_method(node: Node<'_>, class_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. CC-driven emit shells; parallel shape is intentional.
    if let Some(body) = node.child_by_field_name("body")
        && let Some(name) = method_name(node, class_name, ctx.source_bytes)
    {
        push_form(body, &name, ctx);
    }
}

fn maybe_emit_arrow(node: Node<'_>, parent_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    if let Some(body) = statement_body(node) {
        let name = parent_name.map_or_else(|| anonymous_name(None, "$arrow", node), str::to_owned);
        push_form(body, &name, ctx);
    }
}

fn maybe_emit_function_expression(
    node: Node<'_>,
    parent_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) {
    if let Some(body) = node.child_by_field_name("body") {
        let name = named_or_anonymous(node, parent_name, "$function", ctx.source_bytes);
        push_form(body, &name, ctx);
    }
}
