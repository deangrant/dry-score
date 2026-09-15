//! Extract Python functions, methods, and lambdas from a CST.

mod forms;
mod names;

use std::path::Path;

use dry_core::{
    FormKind, FormSpan, NormalizedForm, PlaceholderMap, below_size_thresholds, fingerprint_tree,
};
use tree_sitter::Node;

use super::emit::{emit_node, should_skip};
use super::kind_from_path;
use super::suppress::span_is_ignored;
use forms::{try_walk_decorated, try_walk_function, try_walk_lambda};

/// Walks a parsed file and emits size-filtered forms.
pub(super) fn extract_forms(
    root: Node<'_>,
    path: &Path,
    source_bytes: &[u8],
    source: &str,
    min_nodes: u32,
    min_lines: u32,
    next_id: &mut u64,
) -> Vec<NormalizedForm> {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
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
    walk_forms(root, None, None, &mut ctx);
    ctx.forms
}

pub(super) struct ExtractCtx<'a> {
    pub path: &'a Path,
    pub source_bytes: &'a [u8],
    pub source: &'a str,
    pub min_nodes: u32,
    pub min_lines: u32,
    pub next_id: &'a mut u64,
    pub kind: FormKind,
    pub forms: Vec<NormalizedForm>,
}

pub(super) fn walk_forms(
    node: Node<'_>,
    parent_name: Option<&str>,
    class_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) {
    // dry-rs:ignore. CC-driven CST kind dispatch; parallel shape is intentional.
    if try_walk_function(node, class_name, ctx) {
        return;
    }
    if try_walk_lambda(node, parent_name, ctx) {
        return;
    }
    if try_walk_decorated(node, class_name, ctx) {
        return;
    }
    walk_children(node, parent_name, class_name, ctx);
}

/// Walks named children, propagating binding / class context from `node`.
pub(super) fn walk_children(
    node: Node<'_>,
    parent_name: Option<&str>,
    class_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    let next_class =
        class_name_from(node, ctx.source_bytes).or_else(|| class_name.map(str::to_owned));
    let next_parent =
        binding_name_from(node, ctx.source_bytes).or_else(|| parent_name.map(str::to_owned));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if should_skip(child) {
            continue;
        }
        walk_forms(child, next_parent.as_deref(), next_class.as_deref(), ctx);
    }
}

fn class_name_from(node: Node<'_>, source: &[u8]) -> Option<String> {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    if node.kind() != "class_definition" {
        return None;
    }
    node.child_by_field_name("name")
        .map(|n| super::emit::node_text(n, source).to_owned())
}

fn binding_name_from(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() != "assignment" && node.kind() != "pair" {
        return None;
    }
    let field = if node.kind() == "assignment" {
        "left"
    } else {
        "key"
    };
    let left = node.child_by_field_name(field)?;
    if left.kind() == "identifier" || left.kind() == "string" {
        return Some(super::emit::node_text(left, source).to_owned());
    }
    None
}

pub(super) fn push_form(body: Node<'_>, name: &str, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::parse::parse_source;
    use std::path::Path;

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "merged extract corpus covers class, named, and nested lambda forms"
    )]
    fn extracts_class_methods_named_functions_and_nested_lambdas() {
        let class_src = r"class Counter:
    def score_left(self, input, factor):
        total = input
        if total < 0:
            total = 0 - total
        scaled = total * factor
        if scaled > 100:
            return scaled - 10
        return scaled + 1

def host(n):
    if n < 0:
        return 0 - n
    return n + 1
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(class_src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("run.py"),
            class_src.as_bytes(),
            class_src,
            3,
            2,
            &mut next_id,
        );
        assert!(
            forms.iter().any(|f| f.name == "Counter::score_left"),
            "forms={forms:?}"
        );
        assert!(forms.iter().any(|f| f.name == "host"), "forms={forms:?}");

        let nested_src = r"def with_closures():
    left = lambda n: (
        (0 - n if n < 0 else n) + 1
    )
    right = lambda n: (
        (0 - n if n < 0 else n) + 1
    )
    return left(3) + right(4)
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let nested_tree = parse_source(nested_src).expect("parse");
        let nested_forms = extract_forms(
            nested_tree.tree.root_node(),
            Path::new("lit.py"),
            nested_src.as_bytes(),
            nested_src,
            3,
            2,
            &mut next_id,
        );
        assert!(
            nested_forms.iter().any(|f| f.name.contains("left")),
            "lambda forms={nested_forms:?}"
        );
        assert!(
            nested_forms.iter().any(|f| f.name.contains("right")),
            "lambda forms={nested_forms:?}"
        );
    }

    #[test]
    fn kind_from_test_file_and_ignore_span() {
        assert_eq!(kind_from_path(Path::new("test_foo.py")), FormKind::Test);
        assert_eq!(kind_from_path(Path::new("foo_test.py")), FormKind::Test);
        assert_eq!(kind_from_path(Path::new("tests/foo.py")), FormKind::Test);
        assert_eq!(kind_from_path(Path::new("test/foo.py")), FormKind::Test);
        assert_eq!(kind_from_path(Path::new("foo.py")), FormKind::Production);
        let src = r"def kept(n):
    if n < 0:
        # dry-py:ignore
        return 0 - n
    return n + 1
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("kept.py"),
            src.as_bytes(),
            src,
            3,
            2,
            &mut next_id,
        );
        assert!(forms.is_empty(), "ignored span should drop form: {forms:?}");
    }

    #[test]
    fn below_thresholds_drops_tiny_forms() {
        let tiny = "def tiny():\n    return 1\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(tiny).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("tiny.py"),
            tiny.as_bytes(),
            tiny,
            999,
            999,
            &mut next_id,
        );
        assert!(forms.is_empty(), "below thresholds: {forms:?}");
    }

    #[test]
    fn parent_bags_stub_nested_lambdas() {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
        let src = parent_stub_py_source();
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("stub.py"),
            src.as_bytes(),
            src,
            3,
            2,
            &mut next_id,
        );
        #[expect(clippy::expect_used, reason = "test asserts extract found parents")]
        let alpha = forms.iter().find(|f| f.name == "alpha").expect("alpha");
        #[expect(clippy::expect_used, reason = "test asserts extract found parents")]
        let beta = forms.iter().find(|f| f.name == "beta").expect("beta");
        let score = dry_core::compare::jaccard(&alpha.fingerprints, &beta.fingerprints);
        assert!(
            score < 0.85,
            "parent bags should differ without nested inflation; score={score}"
        );
        let lambdas: Vec<_> = forms.iter().filter(|f| f.name == "c").collect();
        assert!(lambdas.len() >= 2, "expected nested lambdas, got {forms:?}");
        assert_eq!(lambdas[0].fingerprints, lambdas[1].fingerprints);
    }

    #[test]
    fn parent_bags_ignore_nested_body_differences() {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
        let src = parent_ignore_nested_py_source();
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("stub.py"),
            src.as_bytes(),
            src,
            3,
            2,
            &mut next_id,
        );
        #[expect(clippy::expect_used, reason = "test asserts extract found parents")]
        let left = forms.iter().find(|f| f.name == "left").expect("left");
        #[expect(clippy::expect_used, reason = "test asserts extract found parents")]
        let right = forms.iter().find(|f| f.name == "right").expect("right");
        assert_eq!(
            left.fingerprints, right.fingerprints,
            "stubbed parents should match when only nested bodies differ"
        );
        let lambdas: Vec<_> = forms.iter().filter(|f| f.name == "c").collect();
        assert!(lambdas.len() >= 2, "expected nested lambdas, got {forms:?}");
        assert_ne!(lambdas[0].fingerprints, lambdas[1].fingerprints);
    }

    fn parent_ignore_nested_py_source() -> &'static str {
        r"def left():
    c = lambda n: (
        (0 - n if n < 0 else n) + 1
    )
    x = 1
    y = x + 2
    z = y + 3
    return c(z)

def right():
    c = lambda n: (
        n * 2 if n > 0 else 0
    )
    x = 1
    y = x + 2
    z = y + 3
    return c(z)
"
    }

    fn parent_stub_py_source() -> &'static str {
        r"def alpha():
    c = lambda n: (
        (0 - n if n < 0 else n) + 1
    )
    x = 1
    y = x + 2
    z = y + 3
    return c(z)

def beta():
    c = lambda n: (
        (0 - n if n < 0 else n) + 1
    )
    a = 10
    b = a * 2
    d = b - 1
    return c(d)
"
    }
}
