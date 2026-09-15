//! Naming helpers for extracted TypeScript forms.

use tree_sitter::Node;

use crate::normalize::emit::node_text;

pub(super) fn function_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
    node.child_by_field_name("name").map(|n| node_text(n, source).to_owned())
}

pub(super) fn method_name(
    node: Node<'_>,
    class_name: Option<&str>,
    source: &[u8],
) -> Option<String> {
    let method = node.child_by_field_name("name").map(|n| node_text(n, source))?;
    let owner = class_name.unwrap_or("_");
    Some(format!("{owner}::{method}"))
}

pub(super) fn anonymous_name(parent_name: Option<&str>, kind: &str, node: Node<'_>) -> String {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
    let line = u32::try_from(node.start_position().row.saturating_add(1)).unwrap_or(u32::MAX);
    parent_name.map_or_else(
        || format!("{kind}:L{line}"),
        |parent| format!("{parent}.{kind}:L{line}"),
    )
}

pub(super) fn named_or_anonymous(
    node: Node<'_>,
    parent_name: Option<&str>,
    kind: &str,
    source: &[u8],
) -> String {
    if let Some(name) = function_name(node, source) {
        return parent_name.map_or_else(|| name.clone(), |parent| format!("{parent}.{name}"));
    }
    if let Some(parent) = parent_name {
        return parent.to_owned();
    }
    anonymous_name(None, kind, node)
}
