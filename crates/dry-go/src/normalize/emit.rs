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
    emit_branch(node, source, placeholders)
}

fn emit_branch(node: Node<'_>, source: &[u8], placeholders: &mut PlaceholderMap) -> NormNode {
    let label = structural_label(node, source);
    let children = emit_children(node, source, placeholders);
    if children.is_empty() {
        NormNode::leaf(label)
    } else {
        NormNode::branch(label, children)
    }
}

fn emit_children(
    node: Node<'_>,
    source: &[u8],
    placeholders: &mut PlaceholderMap,
) -> Vec<NormNode> {
    let mut children = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if should_skip(child) {
            continue;
        }
        children.push(emit_node(child, source, placeholders));
    }
    children
}

pub(super) fn should_skip(node: Node<'_>) -> bool {
    !node.is_named() || node.kind() == "comment" || node.is_error() || node.is_missing()
}

fn try_emit_leaf(
    node: Node<'_>,
    source: &[u8],
    placeholders: &mut PlaceholderMap,
) -> Option<NormNode> {
    // dry-rs:ignore. CC-driven leaf family dispatch; parallel shape is intentional.
    try_emit_ident_leaf(node, source, placeholders)
        .or_else(|| try_emit_number_leaf(node))
        .or_else(|| try_emit_text_leaf(node))
        .or_else(|| try_emit_keyword_leaf(node))
}

fn try_emit_ident_leaf(
    node: Node<'_>,
    source: &[u8],
    placeholders: &mut PlaceholderMap,
) -> Option<NormNode> {
    match node.kind() {
        "identifier" | "field_identifier" | "package_identifier" | "type_identifier" => {
            let text = node_text(node, source);
            Some(NormNode::leaf(placeholders.placeholder(text)))
        }
        _ => None,
    }
}

fn try_emit_number_leaf(node: Node<'_>) -> Option<NormNode> {
    if node.kind() == "int_literal" {
        return Some(NormNode::leaf("lit_int"));
    }
    try_emit_non_int_number_leaf(node.kind())
}

fn try_emit_non_int_number_leaf(kind: &str) -> Option<NormNode> {
    match kind {
        "float_literal" => Some(NormNode::leaf("lit_float")),
        "imaginary_literal" => Some(NormNode::leaf("lit_imag")),
        "rune_literal" => Some(NormNode::leaf("lit_rune")),
        _ => None,
    }
}

fn try_emit_text_leaf(node: Node<'_>) -> Option<NormNode> {
    match node.kind() {
        "interpreted_string_literal" | "raw_string_literal" => Some(NormNode::leaf("lit_str")),
        _ => None,
    }
}

fn try_emit_keyword_leaf(node: Node<'_>) -> Option<NormNode> {
    match node.kind() {
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
    node.children(&mut cursor)
        .filter(|child| !child.is_named())
        .map(|child| node_text(child, source).trim().to_owned())
        .find(|text| !text.is_empty())
        .unwrap_or_else(|| "_".to_owned())
}

/// UTF-8 slice for a CST node's byte range.
///
/// Invalid UTF-8 (for example a Tree-sitter range that splits a codepoint on a
/// partial CST) yields U+FFFD instead of an empty string so names and
/// placeholders stay distinguishable.
#[must_use]
pub(super) fn node_text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    decode_node_bytes(&source[node.byte_range()])
}

fn decode_node_bytes(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap_or("\u{FFFD}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::parse::parse_source;

    #[test]
    fn invalid_utf8_bytes_use_replacement() {
        assert_eq!(decode_node_bytes(&[0xff, 0xfe]), "\u{FFFD}");
        assert_eq!(decode_node_bytes(b"ok"), "ok");
        assert_eq!(decode_node_bytes(b""), "");
    }

    #[test]
    fn emit_renames_idents_consistently() {
        let src = "package p\nfunc add(a int, b int) int { return a + b }\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let root = tree.tree.root_node();
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

    #[test]
    fn emit_covers_literals_keywords_and_comments() {
        let src = r#"package p
func demo() {
  // skip me
  x := 1.5
  y := "hi"
  z := 'a'
  _ = true
  _ = nil
  _ = 1i
}
"#;
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let mut placeholders = PlaceholderMap::default();
        let _ = emit_node(tree.tree.root_node(), src.as_bytes(), &mut placeholders);
        assert!(placeholders.ident_trace.iter().any(|s| s == "demo" || s == "x"));
    }

    #[test]
    fn emit_skips_comments_and_unknown_operators() {
        let src = "package p\nfunc demo() { /* c */ x := 1 }\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let root = tree.tree.root_node();
        let mut comment = None;
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            walk_find_comment(child, &mut comment);
        }
        if let Some(node) = comment {
            let mut placeholders = PlaceholderMap::default();
            let skipped = emit_node(node, src.as_bytes(), &mut placeholders);
            assert_eq!(skipped.label, "skip");
        }
        assert_eq!(operator_text(root, src.as_bytes()), "_");
    }

    fn walk_find_comment<'a>(node: Node<'a>, found: &mut Option<Node<'a>>) {
        if found.is_some() {
            return;
        }
        if node.kind() == "comment" {
            *found = Some(node);
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk_find_comment(child, found);
        }
    }
}
