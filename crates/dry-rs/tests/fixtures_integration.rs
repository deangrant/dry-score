//! Integration tests against fixture corpora.

use std::path::PathBuf;

use dry_core::{CloneType, Config, OutputFormat, Report, analyze};
use dry_rs::RustNormalizer;

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn analyze_fixture(name: &str, threshold: f64) -> Report {
    let mut config = Config::default();
    config.gate.threshold = threshold;
    config.walk.min_nodes = 8;
    config.walk.min_lines = 3;
    config.output.format = OutputFormat::Json;
    let normalizer = RustNormalizer::new(config.walk.min_nodes, config.walk.min_lines);
    let roots = vec![fixture_dir(name)];
    let result = analyze(&roots, &config, &normalizer);
    assert!(
        result.is_ok(),
        "analyze failed: {}",
        result.as_ref().err().map_or("-", String::as_str)
    );
    #[expect(
        clippy::expect_used,
        reason = "test helper surfaces analysis errors as assertion failures"
    )]
    result.expect("analyze ok").report
}

fn assert_first_clone(report: &Report, expected: CloneType, exact_score: bool) {
    assert!(!report.findings.is_empty(), "expected findings, got none");
    let finding = &report.findings[0];
    assert_eq!(finding.clone_type, expected);
    if exact_score {
        assert!((finding.score - 1.0).abs() < f64::EPSILON);
    } else {
        assert!(finding.score < 1.0);
        assert!(finding.score >= 0.5);
    }
}

#[test]
fn type_1_exact_is_auto_refactor() {
    assert_first_clone(
        &analyze_fixture("type_1_exact", 0.85),
        CloneType::Type1,
        true,
    );
}

#[test]
fn type_2_renamed_is_type_two() {
    assert_first_clone(
        &analyze_fixture("type_2_renamed", 0.85),
        CloneType::Type2,
        true,
    );
}

#[test]
fn type_3_near_miss_is_type_three() {
    assert_first_clone(
        &analyze_fixture("type_3_near_miss", 0.5),
        CloneType::Type3,
        false,
    );
}

#[test]
fn non_clone_has_no_findings() {
    let report = analyze_fixture("non_clone", 0.85);
    assert!(
        report.findings.is_empty(),
        "unexpected findings: {:?}",
        report.findings
    );
}
