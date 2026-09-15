//! Naming helpers for extracted Python forms.

use tree_sitter::Node;

use crate::normalize::emit::node_text;

pub(super) fn function_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name").map(|n| node_text(n, source).to_owned())
}

pub(super) fn method_name(
    node: Node<'_>,
    class_name: Option<&str>,
    source: &[u8],
) -> Option<String> {
    let method = function_name(node, source)?;
    match class_name {
        Some(owner) => Some(format!("{owner}::{method}")),
        None => Some(method),
    }
}

pub(super) fn anonymous_name(parent_name: Option<&str>, kind: &str, node: Node<'_>) -> String {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    let line = u32::try_from(node.start_position().row.saturating_add(1)).unwrap_or(u32::MAX);
    parent_name.map_or_else(
        || format!("{kind}:L{line}"),
        |parent| format!("{parent}.{kind}:L{line}"),
    )
}

pub(super) fn lambda_name(parent_name: Option<&str>, node: Node<'_>) -> String {
    parent_name.map_or_else(|| anonymous_name(None, "$lambda", node), str::to_owned)
}
