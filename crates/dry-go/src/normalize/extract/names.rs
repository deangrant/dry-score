//! Naming helpers for extracted Go forms.

use tree_sitter::Node;

use crate::normalize::emit::node_text;

pub(super) fn function_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name").map(|n| node_text(n, source).to_owned())
}

pub(super) fn method_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let method = node.child_by_field_name("name").map(|n| node_text(n, source))?;
    let recv = receiver_type_name(node, source).unwrap_or_else(|| "_".to_owned());
    Some(format!("{recv}::{method}"))
}

pub(super) fn literal_name(parent_name: Option<&str>, node: Node<'_>) -> String {
    let line = u32::try_from(node.start_position().row.saturating_add(1)).unwrap_or(u32::MAX);
    parent_name.map_or_else(
        || format!("$literal:L{line}"),
        |parent| format!("{parent}.$literal:L{line}"),
    )
}

fn receiver_type_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let params = node.child_by_field_name("receiver")?;
    first_receiver_type(params, source)
}

fn first_receiver_type(params: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = params.walk();
    params.children(&mut cursor).find_map(|child| param_type_name(child, source))
}

fn param_type_name(child: Node<'_>, source: &[u8]) -> Option<String> {
    if child.kind() != "parameter_declaration" {
        return None;
    }
    child.child_by_field_name("type").map(|ty| strip_pointer(ty, source))
}

fn strip_pointer(node: Node<'_>, source: &[u8]) -> String {
    if let Some(inner) = pointer_inner(node) {
        return strip_pointer(inner, source);
    }
    if is_type_name(node) {
        return node_text(node, source).to_owned();
    }
    first_named_child_type(node, source)
}

fn pointer_inner(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "pointer_type" {
        node.named_child(0)
    } else {
        None
    }
}

fn is_type_name(node: Node<'_>) -> bool {
    node.kind() == "type_identifier" || node.kind() == "identifier"
}

fn first_named_child_type(node: Node<'_>, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            return strip_pointer(child, source);
        }
    }
    node_text(node, source).to_owned()
}
