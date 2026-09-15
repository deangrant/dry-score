//! CST walk helpers that emit Go function-like forms.

use tree_sitter::Node;

use super::names::{function_name, literal_name, method_name};
use super::{ExtractCtx, push_form, walk_children};

pub(super) fn try_walk_function(node: Node<'_>, ctx: &mut ExtractCtx<'_>) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    if node.kind() != "function_declaration" {
        return false;
    }
    maybe_emit_function(node, ctx);
    let name = function_name(node, ctx.source_bytes);
    walk_children(node, name.as_deref(), ctx);
    true
}

pub(super) fn try_walk_method(node: Node<'_>, ctx: &mut ExtractCtx<'_>) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    if node.kind() != "method_declaration" {
        return false;
    }
    maybe_emit_method(node, ctx);
    let name = method_name(node, ctx.source_bytes);
    walk_children(node, name.as_deref(), ctx);
    true
}

pub(super) fn try_walk_literal(
    node: Node<'_>,
    parent_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) -> bool {
    if node.kind() != "func_literal" {
        return false;
    }
    maybe_emit_literal(node, parent_name, ctx);
    walk_children(node, parent_name, ctx);
    true
}

pub(super) fn maybe_emit_function(node: Node<'_>, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. CC-driven emit shells; parallel shape is intentional.
    if let Some(body) = node.child_by_field_name("body")
        && let Some(name) = function_name(node, ctx.source_bytes)
    {
        push_form(body, &name, ctx);
    }
}

pub(super) fn maybe_emit_method(node: Node<'_>, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. CC-driven emit shells; parallel shape is intentional.
    if let Some(body) = node.child_by_field_name("body")
        && let Some(name) = method_name(node, ctx.source_bytes)
    {
        push_form(body, &name, ctx);
    }
}

pub(super) fn maybe_emit_literal(
    node: Node<'_>,
    parent_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) {
    if let Some(body) = node.child_by_field_name("body") {
        let name = literal_name(parent_name, node);
        push_form(body, &name, ctx);
    }
}
