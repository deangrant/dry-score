//! Extract Go functions, methods, and func literals from a CST.

use std::path::Path;

use dry_core::{
    FormKind, FormSpan, NormalizedForm, PlaceholderMap, below_size_thresholds, fingerprint_tree,
};
use tree_sitter::Node;

use super::emit::{emit_node, node_text, should_skip};
use super::kind_from_path;
use super::suppress::span_is_ignored;

/// Walks a parsed file and emits size-filtered forms.
pub fn extract_forms(
    root: Node<'_>,
    path: &Path,
    source_bytes: &[u8],
    source: &str,
    min_nodes: u32,
    min_lines: u32,
    next_id: &mut u64,
) -> Vec<NormalizedForm> {
    let mut ctx = ExtractCtx {
        path,
        source_bytes,
        source,
        min_nodes,
        min_lines,
        next_id,
        kind: kind_from_path(path),
        forms: Vec::new(),
    };
    walk_forms(root, None, &mut ctx);
    ctx.forms
}

struct ExtractCtx<'a> {
    path: &'a Path,
    source_bytes: &'a [u8],
    source: &'a str,
    min_nodes: u32,
    min_lines: u32,
    next_id: &'a mut u64,
    kind: FormKind,
    forms: Vec<NormalizedForm>,
}

fn walk_forms(node: Node<'_>, parent_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. CC-driven CST kind dispatch; parallel shape is intentional.
    if try_walk_function(node, ctx) || try_walk_method(node, ctx) {
        return;
    }
    if try_walk_literal(node, parent_name, ctx) {
        return;
    }
    walk_children(node, parent_name, ctx);
}

fn try_walk_function(node: Node<'_>, ctx: &mut ExtractCtx<'_>) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    if node.kind() != "function_declaration" {
        return false;
    }
    maybe_emit_function(node, ctx);
    let name = function_name(node, ctx.source_bytes);
    walk_children(node, name.as_deref(), ctx);
    true
}

fn try_walk_method(node: Node<'_>, ctx: &mut ExtractCtx<'_>) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    if node.kind() != "method_declaration" {
        return false;
    }
    maybe_emit_method(node, ctx);
    let name = method_name(node, ctx.source_bytes);
    walk_children(node, name.as_deref(), ctx);
    true
}

fn try_walk_literal(node: Node<'_>, parent_name: Option<&str>, ctx: &mut ExtractCtx<'_>) -> bool {
    if node.kind() != "func_literal" {
        return false;
    }
    maybe_emit_literal(node, parent_name, ctx);
    walk_children(node, parent_name, ctx);
    true
}

fn walk_children(node: Node<'_>, parent_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if should_skip(child) {
            continue;
        }
        walk_forms(child, parent_name, ctx);
    }
}

fn maybe_emit_function(node: Node<'_>, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. CC-driven emit shells; parallel shape is intentional.
    if let Some(body) = node.child_by_field_name("body")
        && let Some(name) = function_name(node, ctx.source_bytes)
    {
        push_form(body, &name, ctx);
    }
}

fn maybe_emit_method(node: Node<'_>, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. CC-driven emit shells; parallel shape is intentional.
    if let Some(body) = node.child_by_field_name("body")
        && let Some(name) = method_name(node, ctx.source_bytes)
    {
        push_form(body, &name, ctx);
    }
}

fn maybe_emit_literal(node: Node<'_>, parent_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    if let Some(body) = node.child_by_field_name("body") {
        let line = u32::try_from(node.start_position().row.saturating_add(1)).unwrap_or(u32::MAX);
        let name = parent_name.map_or_else(
            || format!("$literal:L{line}"),
            |parent| format!("{parent}.$literal:L{line}"),
        );
        push_form(body, &name, ctx);
    }
}

fn push_form(body: Node<'_>, name: &str, ctx: &mut ExtractCtx<'_>) {
    let start = u32::try_from(body.start_position().row.saturating_add(1)).unwrap_or(1);
    let end = u32::try_from(body.end_position().row.saturating_add(1)).unwrap_or(start);
    if span_is_ignored(ctx.source, start, end) {
        return;
    }
    let mut placeholders = PlaceholderMap::default();
    let tree = emit_node(body, ctx.source_bytes, &mut placeholders);
    let fp = fingerprint_tree(&tree);
    if below_size_thresholds(fp.node_count, start, end, ctx.min_nodes, ctx.min_lines) {
        return;
    }
    let id = *ctx.next_id;
    *ctx.next_id = ctx.next_id.saturating_add(1);
    ctx.forms.push(NormalizedForm {
        id,
        name: name.to_owned(),
        path: ctx.path.to_path_buf(),
        span: FormSpan {
            start_line: start,
            end_line: end,
        },
        kind: ctx.kind,
        node_count: fp.node_count,
        fingerprints: fp.fingerprints,
        ident_trace: placeholders.ident_trace,
    });
}

fn function_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name").map(|n| node_text(n, source).to_owned())
}

fn method_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let method = node.child_by_field_name("name").map(|n| node_text(n, source))?;
    let recv = receiver_type_name(node, source).unwrap_or_else(|| "_".to_owned());
    Some(format!("{recv}::{method}"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::parse::parse_source;
    use std::path::Path;

    #[test]
    fn extracts_pointer_and_value_receiver_methods() {
        let pointer = r"package p
type T struct{}
func (t *T) Run(n int) int {
  if n < 0 {
    n = 0 - n
  }
  return n + 1
}
";
        let value = r"package p
type T struct{}
func (t T) Value(n int) int {
  if n < 0 {
    return 0
  }
  return n + 1
}
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let pointer_tree = parse_source(pointer).expect("parse");
        #[expect(clippy::expect_used, reason = "test setup")]
        let value_tree = parse_source(value).expect("parse");
        let mut next_id = 1;
        let pointer_forms = extract_forms(
            pointer_tree.tree.root_node(),
            Path::new("run.go"),
            pointer.as_bytes(),
            pointer,
            3,
            2,
            &mut next_id,
        );
        let value_forms = extract_forms(
            value_tree.tree.root_node(),
            Path::new("value.go"),
            value.as_bytes(),
            value,
            3,
            2,
            &mut next_id,
        );
        assert!(
            pointer_forms.iter().any(|f| f.name == "T::Run"),
            "forms={pointer_forms:?}"
        );
        assert!(
            value_forms.iter().any(|f| f.name == "T::Value"),
            "forms={value_forms:?}"
        );
    }

    #[test]
    fn kind_from_test_file_and_ignore_span() {
        assert_eq!(kind_from_path(Path::new("foo_test.go")), FormKind::Test);
        let src = r"package p
func kept(n int) int {
  // dry-go:ignore
  if n < 0 {
    return 0 - n
  }
  return n + 1
}
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("kept.go"),
            src.as_bytes(),
            src,
            3,
            2,
            &mut next_id,
        );
        assert!(forms.is_empty(), "ignored span should drop form: {forms:?}");
    }

    #[test]
    fn extracts_interface_receiver_methods() {
        let src = r"package p
func (x interface{}) Run(n int) int {
  if n < 0 {
    n = 0 - n
  }
  return n + 1
}
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("iface.go"),
            src.as_bytes(),
            src,
            3,
            2,
            &mut next_id,
        );
        assert!(
            forms.iter().any(|f| f.name.contains("::Run")),
            "forms={forms:?}"
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "coverage corpus hits literal, soft-parse, and map-receiver paths"
    )]
    fn extract_coverage_edge_cases() {
        let tiny = "package p\nfunc tiny() int { return 1 }\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(tiny).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("tiny.go"),
            tiny.as_bytes(),
            tiny,
            999,
            999,
            &mut next_id,
        );
        assert!(forms.is_empty(), "below thresholds: {forms:?}");

        let literal = r"package p
var top = func(n int) int {
  if n < 0 {
    return 0 - n
  }
  return n + 1
}
func host() {
  f := func(n int) int {
    if n < 0 {
      return 0 - n
    }
    return n + 1
  }
  _ = f(1)
}
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(literal).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("lit.go"),
            literal.as_bytes(),
            literal,
            3,
            2,
            &mut next_id,
        );
        assert!(
            forms.iter().any(|f| f.name.starts_with("$literal:L")),
            "top-level literal forms={forms:?}"
        );
        assert!(
            forms.iter().any(|f| f.name.contains("host.$literal:")),
            "parented literal forms={forms:?}"
        );

        let broken = "package p\nfunc (\nfunc { }\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(broken).expect("parse");
        let mut next_id = 1;
        let _ = extract_forms(
            tree.tree.root_node(),
            Path::new("broken.go"),
            broken.as_bytes(),
            broken,
            1,
            1,
            &mut next_id,
        );

        let maprecv = r"package p
func (x map[string]int) Run(n int) int {
  if n < 0 {
    return 0
  }
  return n + 1
}
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(maprecv).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("maprecv.go"),
            maprecv.as_bytes(),
            maprecv,
            3,
            2,
            &mut next_id,
        );
        assert!(
            forms.iter().any(|f| f.name.contains("::Run")),
            "map receiver forms={forms:?}"
        );
        let _ = should_skip(tree.tree.root_node());
    }

    #[test]
    fn maybe_emit_skips_nodes_without_body() {
        let src = "package p\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let root = tree.tree.root_node();
        let mut next_id = 1;
        let mut ctx = ExtractCtx {
            path: Path::new("p.go"),
            source_bytes: src.as_bytes(),
            source: src,
            min_nodes: 1,
            min_lines: 1,
            next_id: &mut next_id,
            kind: FormKind::Production,
            forms: Vec::new(),
        };
        maybe_emit_function(root, &mut ctx);
        maybe_emit_method(root, &mut ctx);
        maybe_emit_literal(root, None, &mut ctx);
        assert!(ctx.forms.is_empty());
    }
}
