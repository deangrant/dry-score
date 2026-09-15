//! Unit tests for Go form extraction.

use std::path::Path;

use dry_core::FormKind;

use super::forms::{maybe_emit_function, maybe_emit_literal, maybe_emit_method};
use super::{ExtractCtx, extract_forms};
use crate::normalize::emit::should_skip;
use crate::normalize::kind_from_path;
use crate::normalize::parse::parse_source;

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

#[test]
fn parent_bags_stub_nested_func_literals() {
    let src = parent_stub_go_source();
    #[expect(clippy::expect_used, reason = "test setup")]
    let tree = parse_source(src).expect("parse");
    let mut next_id = 1;
    let forms = extract_forms(
        tree.tree.root_node(),
        Path::new("stub.go"),
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
    let literals: Vec<_> = forms.iter().filter(|f| f.name.contains("$literal:")).collect();
    assert!(
        literals.len() >= 2,
        "expected nested literals, got {forms:?}"
    );
    assert_eq!(literals[0].fingerprints, literals[1].fingerprints);
}

fn parent_stub_go_source() -> &'static str {
    r"package p
func alpha() int {
  c := func(n int) int {
    acc := n
    if acc < 0 {
      acc = 0 - acc
    }
    return acc + 1
  }
  x := 1
  y := x + 2
  z := y + 3
  return c(z)
}
func beta() int {
  c := func(n int) int {
    acc := n
    if acc < 0 {
      acc = 0 - acc
    }
    return acc + 1
  }
  a := 10
  b := a * 2
  d := b - 1
  return c(d)
}
"
}
