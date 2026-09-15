//! Unit tests for Python form extraction.

use std::path::Path;

use dry_core::FormKind;

use super::extract_forms;
use crate::normalize::kind_from_path;
use crate::normalize::parse::parse_source;

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

    let pair_src = r#"def with_pairs():
    funcs = {
        "left": lambda n: (
            (0 - n if n < 0 else n) + 1
        ),
    }
    return funcs["left"](3)
"#;
    #[expect(clippy::expect_used, reason = "test setup")]
    let pair_tree = parse_source(pair_src).expect("parse");
    let pair_forms = extract_forms(
        pair_tree.tree.root_node(),
        Path::new("pair.py"),
        pair_src.as_bytes(),
        pair_src,
        3,
        2,
        &mut next_id,
    );
    assert!(
        pair_forms
            .iter()
            .any(|f| f.name.contains("left") || f.name.contains("\"left\"")),
        "pair binding forms={pair_forms:?}"
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
fn parent_bags_stub_nested_forms() {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    for (src, label) in [
        (parent_stub_py_source(), "lambdas"),
        (parent_stub_named_py_source(), "named functions"),
    ] {
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
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    for (src, label) in [
        (parent_ignore_nested_py_source(), "lambdas"),
        (parent_ignore_nested_named_py_source(), "named functions"),
    ] {
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

fn parent_ignore_nested_named_py_source() -> &'static str {
    r"def left():
    def c(n):
        acc = n
        if acc < 0:
            acc = 0 - acc
        return acc + 1
    x = 1
    y = x + 2
    z = y + 3
    return c(z)

def right():
    def c(n):
        acc = n
        while acc > 0:
            acc = acc - 1
        return acc * 2
    x = 1
    y = x + 2
    z = y + 3
    return c(z)
"
}

fn parent_stub_named_py_source() -> &'static str {
    r"def alpha():
    def c(n):
        acc = n
        if acc < 0:
            acc = 0 - acc
        return acc + 1
    x = 1
    y = x + 2
    z = y + 3
    return c(z)

def beta():
    def c(n):
        acc = n
        if acc < 0:
            acc = 0 - acc
        return acc + 1
    a = 10
    b = a * 2
    d = b - 1
    return c(d)
"
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
