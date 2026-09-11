//! Convert Tree-sitter Go CST nodes into [`NormNode`] trees.

use dry_core::{NormNode, PlaceholderMap};
use tree_sitter::Node;

/// Emits a normalized tree for `node` and its significant children.
#[must_use]
pub fn emit_node(node: Node<'_>, source: &[u8], placeholders: &mut PlaceholderMap) -> NormNode {
    if should_skip(node) {
        return NormNode::leaf("skip");
    }
    if let Some(leaf) = try_emit_leaf(node, source, placeholders) {
        return leaf;
    }
    let label = structural_label(node, source);
    let mut children = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if should_skip(child) {
            continue;
        }
        children.push(emit_node(child, source, placeholders));
    }
    if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    }
}

fn should_skip(node: Node<'_>) -> bool {
    !node.is_named() || node.kind() == "comment"
}

fn try_emit_leaf(
    node: Node<'_>,
    source: &[u8],
    placeholders: &mut PlaceholderMap,
) -> Option<NormNode> {
    match node.kind() {
        "identifier" | "field_identifier" | "package_identifier" | "type_identifier" => {
            let text = node_text(node, source);
            Some(NormNode::leaf(placeholders.placeholder(text)))
        }
        "int_literal" => Some(NormNode::leaf("lit_int")),
        "float_literal" => Some(NormNode::leaf("lit_float")),
        "imaginary_literal" => Some(NormNode::leaf("lit_imag")),
        "rune_literal" => Some(NormNode::leaf("lit_rune")),
        "interpreted_string_literal" | "raw_string_literal" => Some(NormNode::leaf("lit_str")),
        "true" | "false" | "nil" | "iota" => Some(NormNode::leaf(node.kind())),
        _ => None,
    }
}

fn structural_label(node: Node<'_>, source: &[u8]) -> String {
    match node.kind() {
        "binary_expression" => format!("bin:{}", operator_text(node, source)),
        "unary_expression" => format!("unary:{}", operator_text(node, source)),
        "assignment_statement" => format!("assign:{}", operator_text(node, source)),
        other => other.to_owned(),
    }
}

fn operator_text(node: Node<'_>, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            let text = node_text(child, source).trim();
            if !text.is_empty() {
                return text.to_owned();
            }
        }
    }
    "_".to_owned()
}

/// UTF-8 slice for a CST node's byte range.
#[must_use]
pub(super) fn node_text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    std::str::from_utf8(&source[node.byte_range()]).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::parse::parse_source;

    #[test]
    fn emit_renames_idents_consistently() {
        let src = "package p\nfunc add(a int, b int) int { return a + b }\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let root = tree.root_node();
        let mut func = None;
        let mut c = root.walk();
        for child in root.children(&mut c) {
            if child.kind() == "function_declaration" {
                func = Some(child);
            }
        }
        #[expect(clippy::expect_used, reason = "test setup")]
        let func = func.expect("func");
        #[expect(clippy::expect_used, reason = "test setup")]
        let body = func.child_by_field_name("body").expect("body");
        let mut placeholders = PlaceholderMap::default();
        let node = emit_node(body, src.as_bytes(), &mut placeholders);
        assert_eq!(node.label, "block");
        assert!(!placeholders.ident_trace.is_empty());
    }
}
