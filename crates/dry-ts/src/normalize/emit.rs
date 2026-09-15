//! Convert Tree-sitter TypeScript CST nodes into [`NormNode`] trees.

use dry_core::{NormNode, PlaceholderMap};
use tree_sitter::Node;

/// Emits a normalized tree for `node` and its significant children.
#[must_use]
pub(super) fn emit_node(
    node: Node<'_>,
    source: &[u8],
    placeholders: &mut PlaceholderMap,
) -> NormNode {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
    if should_skip(node) {
        return NormNode::leaf("skip");
    }
    if is_nested_form_kind(node.kind()) {
        // Nested arrows and named functions are extracted as their own forms; stub here.
        return NormNode::leaf(node.kind());
    }
    if let Some(leaf) = try_emit_leaf(node, source, placeholders) {
        return leaf;
    }
    emit_branch(node, source, placeholders)
}

fn is_nested_form_kind(kind: &str) -> bool {
    matches!(
        kind,
        "arrow_function"
            | "function_expression"
            | "generator_function"
            | "function_declaration"
            | "generator_function_declaration"
    )
}

fn emit_branch(node: Node<'_>, source: &[u8], placeholders: &mut PlaceholderMap) -> NormNode {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
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
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
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
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
    !node.is_named() || node.kind() == "comment" || node.is_error() || node.is_missing()
}

fn try_emit_leaf(
    node: Node<'_>,
    source: &[u8],
    placeholders: &mut PlaceholderMap,
) -> Option<NormNode> {
    // dry-rs:ignore. CC-driven leaf family dispatch; parallel shape is intentional.
    try_emit_ident_leaf(node, source, placeholders)
        .or_else(|| try_emit_number_leaf(node, source))
        .or_else(|| try_emit_text_leaf(node))
        .or_else(|| try_emit_keyword_leaf(node))
}

fn try_emit_ident_leaf(
    node: Node<'_>,
    source: &[u8],
    placeholders: &mut PlaceholderMap,
) -> Option<NormNode> {
    match node.kind() {
        "identifier"
        | "type_identifier"
        | "property_identifier"
        | "shorthand_property_identifier"
        | "private_property_identifier" => {
            let text = node_text(node, source);
            Some(NormNode::leaf(placeholders.placeholder(text)))
        }
        _ => None,
    }
}

fn try_emit_number_leaf(node: Node<'_>, source: &[u8]) -> Option<NormNode> {
    if node.kind() != "number" {
        return None;
    }
    let text = node_text(node, source);
    if text.contains('.') || text.contains('e') || text.contains('E') {
        Some(NormNode::leaf("lit_float"))
    } else {
        Some(NormNode::leaf("lit_int"))
    }
}

fn try_emit_text_leaf(node: Node<'_>) -> Option<NormNode> {
    match node.kind() {
        "string" | "string_fragment" => Some(NormNode::leaf("lit_str")),
        "template_string" => emit_plain_template_leaf(node),
        "regex" => Some(NormNode::leaf("lit_regex")),
        _ => None,
    }
}

fn emit_plain_template_leaf(node: Node<'_>) -> Option<NormNode> {
    (!has_template_substitution(node)).then(|| NormNode::leaf("lit_str"))
}

fn has_template_substitution(node: Node<'_>) -> bool {
    // dry-rs:ignore. Tree-sitter child-walk helper; parallel shape is intentional.
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| child.kind() == "template_substitution")
}

fn try_emit_keyword_leaf(node: Node<'_>) -> Option<NormNode> {
    match node.kind() {
        "true" | "false" | "null" | "undefined" | "this" | "super" => {
            Some(NormNode::leaf(node.kind()))
        }
        _ => None,
    }
}

fn structural_label(node: Node<'_>, source: &[u8]) -> String {
    match node.kind() {
        "binary_expression" => format!("bin:{}", operator_text(node, source)),
        "unary_expression" | "update_expression" => {
            format!("unary:{}", operator_text(node, source))
        }
        "assignment_expression" | "augmented_assignment_expression" => {
            format!("assign:{}", operator_text(node, source))
        }
        other => other.to_owned(),
    }
}

fn operator_text(node: Node<'_>, source: &[u8]) -> String {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
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
    use std::path::Path;

    #[test]
    fn invalid_utf8_bytes_use_replacement() {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
        assert_eq!(decode_node_bytes(&[0xff, 0xfe]), "\u{FFFD}");
        assert_eq!(decode_node_bytes(b"ok"), "ok");
        assert_eq!(decode_node_bytes(b""), "");
    }

    #[test]
    fn emit_renames_idents_consistently() {
        let src = "function add(a: number, b: number): number { return a + b; }\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(Path::new("add.ts"), src).expect("parse");
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
        assert_eq!(node.label, "statement_block");
        assert!(!placeholders.ident_trace.is_empty());
    }

    #[test]
    fn emit_covers_literals_keywords_and_comments() {
        let src = r#"function demo() {
  // skip me
  const x = 1.5;
  const y = "hi";
  const z = /ab+/;
  void true;
  void null;
  void undefined;
  void this;
}
"#;
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(Path::new("demo.ts"), src).expect("parse");
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
        let _ = emit_node(body, src.as_bytes(), &mut placeholders);
        assert!(placeholders.ident_trace.iter().any(|s| s == "x"));
    }

    #[test]
    fn emit_skips_comments_and_unknown_operators() {
        let src = "function demo() { /* c */ const x = 1; }\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(Path::new("demo.ts"), src).expect("parse");
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

    #[test]
    fn interpolated_templates_preserve_substitution_structure() {
        let plain = emit_function_body("function a() { return `hello`; }\n");
        let interp_name = emit_function_body("function a() { return `hi ${user.name}`; }\n");
        let interp_call = emit_function_body("function a() { return `hi ${other.fn()}`; }\n");
        assert_ne!(plain, interp_name);
        assert_ne!(plain, interp_call);
        assert_ne!(interp_name, interp_call);
        assert!(contains_label(&interp_name, "template_string"));
        assert!(contains_label(&interp_name, "template_substitution"));
        assert!(!contains_label(&plain, "template_substitution"));
    }

    fn emit_function_body(src: &str) -> NormNode {
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(Path::new("demo.ts"), src).expect("parse");
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
        emit_node(body, src.as_bytes(), &mut placeholders)
    }

    fn contains_label(node: &NormNode, label: &str) -> bool {
        if node.label == label {
            return true;
        }
        node.children.iter().any(|child| contains_label(child, label))
    }

    fn walk_find_comment<'a>(node: Node<'a>, found: &mut Option<Node<'a>>) {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
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
