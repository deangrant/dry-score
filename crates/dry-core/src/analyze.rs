//! End-to-end analysis orchestration.

use std::fs;
use std::path::{Path, PathBuf};

use crate::compare::compare;
use crate::config::Config;
use crate::domain::{CloneType, Finding, NormalizedForm, ReportSummary, Tier};
use crate::ports::LanguageNormalizer;
use crate::report::Report;
use crate::walk::{WalkOptions, collect_source_files};

/// Result of a full analysis run.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisResult {
    /// Finished report envelope.
    pub report: Report,
}

/// Runs discovery, normalization, comparison, and summary building.
///
/// # Errors
///
/// Returns a string when filesystem discovery fails hard.
pub fn analyze(
    roots: &[PathBuf],
    config: &Config,
    normalizer: &impl LanguageNormalizer,
    tool: &str,
) -> Result<AnalysisResult, String> {
    let options = WalkOptions::new(config.walk.extensions.clone(), config.walk.exclude.clone());
    let files = collect_source_files(roots, &options).map_err(|err| err.to_string())?;
    let (forms, warnings, files_scanned) =
        normalize_sources(&files, roots, normalizer, config.walk.max_file_bytes);
    let forms_compared = u32::try_from(forms.len()).unwrap_or(u32::MAX);
    let findings = compare(&forms, config.gate.threshold);
    let summary = build_summary(
        &findings,
        files_scanned,
        forms_compared,
        u32::try_from(warnings.len()).unwrap_or(u32::MAX),
    );
    let report = Report::new(tool, config.gate.threshold, findings, summary, warnings);
    Ok(AnalysisResult { report })
}

/// Prefer root-relative paths for report stability; keep the original when no
/// root is a prefix.
fn relativize_one(path: &Path, roots: &[PathBuf]) -> PathBuf {
    roots
        .iter()
        .filter_map(|root| {
            path.strip_prefix(root)
                .ok()
                .map(|rel| (root.as_os_str().len(), rel.to_path_buf()))
        })
        .max_by_key(|(len, _)| *len)
        .map_or_else(|| path.to_path_buf(), |(_, rel)| rel)
}

fn normalize_sources(
    files: &[PathBuf],
    roots: &[PathBuf],
    normalizer: &impl LanguageNormalizer,
    max_file_bytes: u64,
) -> (Vec<NormalizedForm>, Vec<String>, u32) {
    let mut forms = Vec::new();
    let mut warnings = Vec::new();
    let mut next_id = 1_u64;
    let mut files_scanned = 0_u32;
    let mut state = NormalizeState {
        next_id: &mut next_id,
        forms: &mut forms,
        warnings: &mut warnings,
        files_scanned: &mut files_scanned,
    };
    for path in files {
        let report_path = relativize_one(path, roots);
        normalize_one(path, &report_path, normalizer, max_file_bytes, &mut state);
    }
    (forms, warnings, files_scanned)
}

struct NormalizeState<'a> {
    next_id: &'a mut u64,
    forms: &'a mut Vec<NormalizedForm>,
    warnings: &'a mut Vec<String>,
    files_scanned: &'a mut u32,
}

fn normalize_one(
    path: &Path,
    report_path: &Path,
    normalizer: &impl LanguageNormalizer,
    max_file_bytes: u64,
    state: &mut NormalizeState<'_>,
) {
    if let Err(warning) = check_file_size(path, report_path, max_file_bytes) {
        state.warnings.push(warning);
        return;
    }
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            state.warnings.push(format!("{}: {err}", report_path.display()));
            return;
        }
    };
    apply_normalize_outcome(report_path, normalizer, &source, state);
}

fn apply_normalize_outcome(
    report_path: &Path,
    normalizer: &impl LanguageNormalizer,
    source: &str,
    state: &mut NormalizeState<'_>,
) {
    match normalizer.normalize_file(report_path, source, state.next_id) {
        Ok(outcome) => record_normalize_ok(report_path, outcome, state),
        Err(err) => state.warnings.push(format!("{}: {err}", report_path.display())),
    }
}

fn record_normalize_ok(
    report_path: &Path,
    outcome: crate::ports::NormalizeOutcome,
    state: &mut NormalizeState<'_>,
) {
    *state.files_scanned = state.files_scanned.saturating_add(1);
    for warning in outcome.warnings {
        state.warnings.push(format!("{}: {warning}", report_path.display()));
    }
    state.forms.extend(outcome.forms);
}

fn check_file_size(path: &Path, report_path: &Path, max_file_bytes: u64) -> Result<(), String> {
    let meta = fs::metadata(path).map_err(|err| format!("{}: {err}", report_path.display()))?;
    let len = meta.len();
    if len > max_file_bytes {
        return Err(format!(
            "{}: file exceeds walk.max_file_bytes ({len} > {max_file_bytes})",
            report_path.display()
        ));
    }
    Ok(())
}

fn build_summary(
    findings: &[Finding],
    files_scanned: u32,
    forms_compared: u32,
    parse_warnings: u32,
) -> ReportSummary {
    let mut summary = ReportSummary {
        files_scanned,
        forms_compared,
        parse_warnings,
        ..ReportSummary::default()
    };
    for finding in findings {
        bump_tier(&mut summary, finding.tier);
        bump_clone_type(&mut summary, finding.clone_type);
    }
    summary
}

const fn bump_tier(summary: &mut ReportSummary, tier: Tier) {
    // dry-rs:ignore. Distinct enum arms; shared saturating_add shape is intentional.
    match tier {
        Tier::AutoRefactor => summary.auto_refactor = summary.auto_refactor.saturating_add(1),
        Tier::ReviewFirst => summary.review_first = summary.review_first.saturating_add(1),
        Tier::Advisory => summary.advisory = summary.advisory.saturating_add(1),
    }
}

const fn bump_clone_type(summary: &mut ReportSummary, clone_type: CloneType) {
    // dry-rs:ignore. Distinct enum arms; shared saturating_add shape is intentional.
    match clone_type {
        CloneType::Type1 => summary.type_1 = summary.type_1.saturating_add(1),
        CloneType::Type2 => summary.type_2 = summary.type_2.saturating_add(1),
        CloneType::Type3 => summary.type_3 = summary.type_3.saturating_add(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FormKind, FormSpan};
    use crate::ports::{NormalizeError, NormalizeOutcome};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct StubNormalizer {
        fail: bool,
        soft_warnings: Vec<String>,
    }

    impl LanguageNormalizer for StubNormalizer {
        fn normalize_file(
            &self,
            path: &Path,
            _source: &str,
            next_id: &mut u64,
        ) -> Result<NormalizeOutcome, NormalizeError> {
            if self.fail {
                return Err(NormalizeError::new("boom"));
            }
            let id = *next_id;
            *next_id = next_id.saturating_add(1);
            Ok(NormalizeOutcome {
                forms: vec![NormalizedForm {
                    id,
                    name: "f".to_owned(),
                    path: path.to_path_buf(),
                    span: FormSpan::new(1, 5),
                    kind: FormKind::Production,
                    node_count: 5,
                    fingerprints: BTreeMap::from([(1, 1), (2, 1), (3, 1)]),
                    ident_trace: vec!["x".to_owned()],
                }],
                warnings: self.soft_warnings.clone(),
            })
        }
    }

    fn temp_rs(label: &str) -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let base = std::env::temp_dir().join(format!("dry-rs-analyze-{label}-{stamp}"));
        assert!(fs::create_dir_all(&base).is_ok());
        let path = base.join("lib.rs");
        assert!(fs::write(&path, "fn a() { let x = 1; }\n").is_ok());
        path
    }

    #[test]
    fn analyze_counts_forms_and_tiers() {
        let path = temp_rs("ok");
        let root = path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let config = Config::default();
        let result = analyze(
            std::slice::from_ref(&root),
            &config,
            &StubNormalizer {
                fail: false,
                soft_warnings: Vec::new(),
            },
            "dry-core",
        );
        assert!(result.is_ok());
        #[expect(clippy::expect_used, reason = "test asserts analyze ok")]
        let result = result.expect("ok");
        assert_eq!(result.report.summary.files_scanned, 1);
        assert_eq!(result.report.summary.forms_compared, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn analyze_emits_root_relative_form_paths() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let root = std::env::temp_dir().join(format!("dry-rs-analyze-relpath-{stamp}"));
        assert!(fs::create_dir_all(&root).is_ok());
        assert!(fs::write(root.join("a.rs"), "fn a() { let x = 1; }\n").is_ok());
        assert!(fs::write(root.join("b.rs"), "fn b() { let y = 2; }\n").is_ok());
        #[expect(clippy::expect_used, reason = "test needs absolute root")]
        let root = root.canonicalize().expect("canonicalize");
        #[expect(clippy::expect_used, reason = "test asserts analyze ok")]
        let result = analyze(
            std::slice::from_ref(&root),
            &Config::default(),
            &StubNormalizer {
                fail: false,
                soft_warnings: Vec::new(),
            },
            "dry-core",
        )
        .expect("ok");
        assert_eq!(result.report.findings.len(), 1);
        assert!(result.report.findings[0].members.iter().all(|m| m.path.is_relative()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn analyze_records_normalize_warnings() {
        let path = temp_rs("warn");
        let root = path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let result = analyze(
            std::slice::from_ref(&root),
            &Config::default(),
            &StubNormalizer {
                fail: true,
                soft_warnings: Vec::new(),
            },
            "dry-core",
        );
        assert!(result.is_ok());
        #[expect(clippy::expect_used, reason = "test asserts analyze ok")]
        let result = result.expect("ok");
        assert_eq!(result.report.parse_warnings.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn analyze_records_soft_normalize_warnings() {
        let path = temp_rs("soft");
        let root = path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let result = analyze(
            std::slice::from_ref(&root),
            &Config::default(),
            &StubNormalizer {
                fail: false,
                soft_warnings: vec!["partial CST".to_owned()],
            },
            "dry-core",
        );
        assert!(result.is_ok());
        #[expect(clippy::expect_used, reason = "test asserts analyze ok")]
        let result = result.expect("ok");
        assert!(result.report.parse_warnings.iter().any(|w| w.contains("partial CST")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn relativize_prefers_longest_matching_root() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let outer = std::env::temp_dir().join(format!("dry-rel-outer-{stamp}"));
        let inner = outer.join("inner");
        assert!(fs::create_dir_all(&inner).is_ok());
        let file = inner.join("lib.rs");
        assert!(fs::write(&file, "fn a() {}\n").is_ok());
        #[expect(clippy::expect_used, reason = "test setup")]
        let outer = outer.canonicalize().expect("outer");
        #[expect(clippy::expect_used, reason = "test setup")]
        let inner = inner.canonicalize().expect("inner");
        #[expect(clippy::expect_used, reason = "test setup")]
        let file = file.canonicalize().expect("file");
        let rel = relativize_one(&file, &[outer.clone(), inner]);
        assert_eq!(rel, PathBuf::from("lib.rs"));
        let unmatched = relativize_one(Path::new("/elsewhere/x.rs"), std::slice::from_ref(&outer));
        assert_eq!(unmatched, PathBuf::from("/elsewhere/x.rs"));
        let _ = fs::remove_dir_all(outer);
    }

    #[test]
    fn analyze_records_unreadable_file_warning() {
        let path = temp_rs("unreadable");
        let root = path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o000);
            assert!(fs::set_permissions(&path, perms).is_ok());
        }
        let result = analyze(
            std::slice::from_ref(&root),
            &Config::default(),
            &StubNormalizer {
                fail: false,
                soft_warnings: Vec::new(),
            },
            "dry-core",
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert!(result.is_ok());
            #[expect(clippy::expect_used, reason = "test asserts analyze ok")]
            let result = result.expect("ok");
            assert!(!result.report.parse_warnings.is_empty());
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
        }
        let _ = result;
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn analyze_missing_root_errors() {
        let missing = PathBuf::from("/no/such/dry-rs-file.rs");
        let err = analyze(
            &[missing],
            &Config::default(),
            &StubNormalizer {
                fail: false,
                soft_warnings: Vec::new(),
            },
            "dry-core",
        );
        assert!(err.is_err());
    }

    #[test]
    fn analyze_skips_oversized_files_with_warning() {
        let path = temp_rs("oversized");
        let root = path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        assert!(fs::write(&path, "fn a() { let x = 1; }\n").is_ok());
        let mut config = Config::default();
        config.walk.max_file_bytes = 1;
        let result = analyze(
            std::slice::from_ref(&root),
            &config,
            &StubNormalizer {
                fail: false,
                soft_warnings: Vec::new(),
            },
            "dry-core",
        );
        assert!(result.is_ok());
        #[expect(clippy::expect_used, reason = "test asserts analyze ok")]
        let result = result.expect("ok");
        assert_eq!(result.report.summary.files_scanned, 0);
        assert_eq!(result.report.summary.forms_compared, 0);
        assert!(result.report.parse_warnings.iter().any(|w| w.contains("max_file_bytes")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_summary_bumps_all_variants() {
        let findings = [
            Finding {
                clone_type: CloneType::Type1,
                tier: Tier::AutoRefactor,
                score: 1.0,
                members: Vec::new(),
            },
            Finding {
                clone_type: CloneType::Type2,
                tier: Tier::ReviewFirst,
                score: 0.9,
                members: Vec::new(),
            },
            Finding {
                clone_type: CloneType::Type3,
                tier: Tier::Advisory,
                score: 0.8,
                members: Vec::new(),
            },
        ];
        let summary = build_summary(&findings, 3, 6, 0);
        assert_eq!(summary.auto_refactor, 1);
        assert_eq!(summary.review_first, 1);
        assert_eq!(summary.advisory, 1);
        assert_eq!(summary.type_1, 1);
        assert_eq!(summary.type_2, 1);
        assert_eq!(summary.type_3, 1);
    }

    #[test]
    fn normalize_error_display() {
        let err = NormalizeError::new("x");
        assert_eq!(err.to_string(), "x");
    }
}
