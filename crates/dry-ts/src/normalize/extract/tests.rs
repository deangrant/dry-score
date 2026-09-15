//! Unit tests for TypeScript form extraction.

use std::path::Path;

use dry_core::FormKind;

use super::extract_forms;
use crate::normalize::kind_from_path;
use crate::normalize::parse::parse_source;

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
fn parent_bags_stub_nested_forms() {
    for (src, label) in [
        (parent_stub_ts_source(), "arrows"),
        (parent_stub_named_ts_source(), "named functions"),
    ] {
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
            "{label}: parent bags should differ without nested inflation; score={score}"
        );
        let nested: Vec<_> = forms.iter().filter(|f| f.name == "c").collect();
        assert!(
            nested.len() >= 2,
            "{label}: expected nested forms, got {forms:?}"
        );
        assert_eq!(nested[0].fingerprints, nested[1].fingerprints);
    }
}

#[test]
fn parent_bags_ignore_nested_body_differences() {
    // Same outer shape, different nested bodies: parents share bags (stub);
    // nested forms diverge. Covers arrows and nested named functions.
    for (src, label) in [
        (parent_ignore_nested_ts_source(), "arrows"),
        (parent_ignore_nested_named_ts_source(), "named functions"),
    ] {
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
            "{label}: stubbed parents should match when only nested bodies differ"
        );
        let nested: Vec<_> = forms.iter().filter(|f| f.name == "c").collect();
        assert!(
            nested.len() >= 2,
            "{label}: expected nested forms, got {forms:?}"
        );
        assert_ne!(nested[0].fingerprints, nested[1].fingerprints);
    }
}

fn parent_ignore_nested_named_ts_source() -> &'static str {
    r"function left(): number {
  function c(n: number): number {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  }
  const x = 1;
  const y = x + 2;
  const z = y + 3;
  return c(z);
}
function right(): number {
  function c(n: number): number {
    let acc = n;
    while (acc > 0) {
      acc = acc - 1;
    }
    return acc * 2;
  }
  const x = 1;
  const y = x + 2;
  const z = y + 3;
  return c(z);
}
"
}

fn parent_stub_named_ts_source() -> &'static str {
    r"function alpha(): number {
  function c(n: number): number {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  }
  const x = 1;
  const y = x + 2;
  const z = y + 3;
  return c(z);
}
function beta(): number {
  function c(n: number): number {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  }
  const a = 10;
  const b = a * 2;
  const d = b - 1;
  return c(d);
}
"
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
