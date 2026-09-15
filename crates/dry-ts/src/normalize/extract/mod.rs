//! Extract TypeScript functions, methods, and arrows from a CST.

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
use forms::{
    try_walk_arrow, try_walk_function, try_walk_function_expression, try_walk_generator,
    try_walk_method,
};

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
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
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
    if try_walk_named_forms(node, class_name, ctx) {
        return;
    }
    if try_walk_expr_forms(node, parent_name, ctx) {
        return;
    }
    walk_children(node, parent_name, class_name, ctx);
}

fn try_walk_named_forms(
    node: Node<'_>,
    class_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    try_walk_function(node, ctx)
        || try_walk_generator(node, ctx)
        || try_walk_method(node, class_name, ctx)
}

fn try_walk_expr_forms(
    node: Node<'_>,
    parent_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    try_walk_arrow(node, parent_name, ctx) || try_walk_function_expression(node, parent_name, ctx)
}

/// Walks named children, propagating binding / class context from `node`.
pub(super) fn walk_children(
    node: Node<'_>,
    parent_name: Option<&str>,
    class_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) {
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
    if node.kind() != "class_declaration" && node.kind() != "class" {
        return None;
    }
    node.child_by_field_name("name")
        .map(|n| super::emit::node_text(n, source).to_owned())
}

fn binding_name_from(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() != "variable_declarator"
        && node.kind() != "pair"
        && node.kind() != "public_field_definition"
    {
        return None;
    }
    node.child_by_field_name("name")
        .map(|n| super::emit::node_text(n, source).to_owned())
}

pub(super) fn push_form(body: Node<'_>, name: &str, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
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
        reason = "merged extract corpus covers class, named, and nested arrow forms"
    )]
    fn extracts_class_methods_named_functions_and_nested_arrows() {
        let class_src = r"class Counter {
  scoreLeft(input: number, factor: number): number {
    let total = input;
    if (total < 0) {
      total = 0 - total;
    }
    const scaled = total * factor;
    if (scaled > 100) {
      return scaled - 10;
    }
    return scaled + 1;
  }
}
function host(n: number): number {
  if (n < 0) {
    return 0 - n;
  }
  return n + 1;
}
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(Path::new("run.ts"), class_src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("run.ts"),
            class_src.as_bytes(),
            class_src,
            3,
            2,
            &mut next_id,
        );
        assert!(
            forms.iter().any(|f| f.name == "Counter::scoreLeft"),
            "forms={forms:?}"
        );
        assert!(forms.iter().any(|f| f.name == "host"), "forms={forms:?}");

        let nested_src = r"function withClosures(): number {
  const left = (n: number): number => {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  };
  const right = function (n: number): number {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  };
  return left(3) + right(4);
}
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let nested_tree = parse_source(Path::new("lit.ts"), nested_src).expect("parse");
        let nested_forms = extract_forms(
            nested_tree.tree.root_node(),
            Path::new("lit.ts"),
            nested_src.as_bytes(),
            nested_src,
            3,
            2,
            &mut next_id,
        );
        assert!(
            nested_forms.iter().any(|f| f.name.contains("left")),
            "arrow forms={nested_forms:?}"
        );
        assert!(
            nested_forms.iter().any(|f| f.name.contains("right")),
            "function expression forms={nested_forms:?}"
        );
    }

    #[test]
    fn kind_from_test_file_and_ignore_span() {
        assert_eq!(kind_from_path(Path::new("foo.test.ts")), FormKind::Test);
        assert_eq!(kind_from_path(Path::new("foo.spec.ts")), FormKind::Test);
        assert_eq!(
            kind_from_path(Path::new("__tests__/foo.ts")),
            FormKind::Test
        );
        assert_eq!(kind_from_path(Path::new("foo.ts")), FormKind::Production);
        let src = r"function kept(n: number): number {
  // dry-ts:ignore
  if (n < 0) {
    return 0 - n;
  }
  return n + 1;
}
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(Path::new("kept.ts"), src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("kept.ts"),
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
        let tiny = "function tiny() { return 1; }\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(Path::new("tiny.ts"), tiny).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("tiny.ts"),
            tiny.as_bytes(),
            tiny,
            999,
            999,
            &mut next_id,
        );
        assert!(forms.is_empty(), "below thresholds: {forms:?}");
    }

    #[test]
    fn parent_bags_stub_nested_arrows() {
        let src = parent_stub_ts_source();
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(Path::new("stub.ts"), src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("stub.ts"),
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
        let arrows: Vec<_> = forms.iter().filter(|f| f.name == "c").collect();
        assert!(arrows.len() >= 2, "expected nested arrows, got {forms:?}");
        assert_eq!(arrows[0].fingerprints, arrows[1].fingerprints);
    }

    #[test]
    fn parent_bags_ignore_nested_body_differences() {
        // Same outer shape, different nested bodies: parents share bags (stub);
        // nested forms diverge.
        let src = parent_ignore_nested_ts_source();
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(Path::new("stub.ts"), src).expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(
            tree.tree.root_node(),
            Path::new("stub.ts"),
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
        let arrows: Vec<_> = forms.iter().filter(|f| f.name == "c").collect();
        assert!(arrows.len() >= 2, "expected nested arrows, got {forms:?}");
        assert_ne!(arrows[0].fingerprints, arrows[1].fingerprints);
    }

    fn parent_ignore_nested_ts_source() -> &'static str {
        r"function left(): number {
  const c = (n: number): number => {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  };
  const x = 1;
  const y = x + 2;
  const z = y + 3;
  return c(z);
}
function right(): number {
  const c = (n: number): number => {
    let acc = n;
    while (acc > 0) {
      acc = acc - 1;
    }
    return acc * 2;
  };
  const x = 1;
  const y = x + 2;
  const z = y + 3;
  return c(z);
}
"
    }

    fn parent_stub_ts_source() -> &'static str {
        r"function alpha(): number {
  const c = (n: number): number => {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  };
  const x = 1;
  const y = x + 2;
  const z = y + 3;
  return c(z);
}
function beta(): number {
  const c = (n: number): number => {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  };
  const a = 10;
  const b = a * 2;
  const d = b - 1;
  return c(d);
}
"
    }
}
