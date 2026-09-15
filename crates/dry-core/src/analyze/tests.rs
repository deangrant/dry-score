//! Unit tests for analysis orchestration.

use super::*;
use crate::domain::{FormKind, FormSpan};
use crate::ports::{NormalizeError, NormalizeOutcome};
use crate::walk::{WalkOptions, collect_source_files};
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
fn analyze_relative_root_still_emits_relative_paths() {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let root = PathBuf::from(format!("target/dry-rs-analyze-relroot-{stamp}"));
    assert!(fs::create_dir_all(&root).is_ok());
    assert!(fs::write(root.join("a.rs"), "fn a() { let x = 1; }\n").is_ok());
    assert!(fs::write(root.join("b.rs"), "fn b() { let y = 2; }\n").is_ok());
    assert!(root.is_relative(), "fixture root must stay relative");
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
    assert!(
        result.report.findings[0].members.iter().all(|m| m.path.is_relative()),
        "members={:?}",
        result.report.findings[0].members
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn analyze_mixed_abs_rel_roots_emit_relative_member_paths() {
    let (abs_root, rel_root) = mixed_abs_rel_fixture();
    #[expect(clippy::expect_used, reason = "test asserts analyze ok")]
    let result = analyze(
        &[abs_root.clone(), rel_root.clone()],
        &Config::default(),
        &StubNormalizer {
            fail: false,
            soft_warnings: Vec::new(),
        },
        "dry-core",
    )
    .expect("ok");
    assert_eq!(result.report.findings.len(), 1);
    assert_mixed_root_members_relative(&result.report.findings[0].members);
    let _ = fs::remove_dir_all(abs_root);
    let _ = fs::remove_dir_all(rel_root);
}

fn mixed_abs_rel_fixture() -> (PathBuf, PathBuf) {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let abs_root = std::env::temp_dir().join(format!("dry-rs-analyze-absroot-{stamp}"));
    let rel_root = PathBuf::from(format!("target/dry-rs-analyze-mixrel-{stamp}"));
    assert!(fs::create_dir_all(&abs_root).is_ok());
    assert!(fs::create_dir_all(&rel_root).is_ok());
    assert!(fs::write(abs_root.join("a.rs"), "fn a() { let x = 1; }\n").is_ok());
    assert!(fs::write(rel_root.join("b.rs"), "fn b() { let y = 2; }\n").is_ok());
    #[expect(clippy::expect_used, reason = "test needs absolute root")]
    let abs_root = abs_root.canonicalize().expect("canonicalize");
    assert!(rel_root.is_relative(), "second root must stay relative");
    (abs_root, rel_root)
}

fn assert_mixed_root_members_relative(members: &[crate::domain::FormMember]) {
    assert!(
        members.iter().all(|m| m.path.is_relative()),
        "members={members:?}"
    );
    let names: Vec<_> = members
        .iter()
        .map(|m| m.path.file_name().map(std::ffi::OsStr::to_owned))
        .collect();
    assert!(names.contains(&Some(std::ffi::OsString::from("a.rs"))));
    assert!(names.contains(&Some(std::ffi::OsString::from("b.rs"))));
}

#[test]
fn normalize_merge_assigns_ids_in_file_order() {
    let (files, root) = two_file_fixture("ids");
    let (forms, warnings, scanned) = normalize_sources(
        &files,
        std::slice::from_ref(&root),
        &StubNormalizer {
            fail: false,
            soft_warnings: vec!["soft".to_owned()],
        },
        u64::MAX,
    );
    assert_eq!(scanned, 2);
    assert_eq!(forms.iter().map(|f| f.id).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(warnings.len(), 2);
    assert!(warnings[0].starts_with(&forms[0].path.display().to_string()));
    assert!(warnings[1].starts_with(&forms[1].path.display().to_string()));
    assert!(warnings.iter().all(|w| w.ends_with(": soft")));
    let _ = fs::remove_dir_all(root);
}

fn two_file_fixture(label: &str) -> (Vec<PathBuf>, PathBuf) {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let root = std::env::temp_dir().join(format!("dry-rs-analyze-{label}-{stamp}"));
    assert!(fs::create_dir_all(&root).is_ok());
    assert!(fs::write(root.join("a.rs"), "fn a() { let x = 1; }\n").is_ok());
    assert!(fs::write(root.join("b.rs"), "fn b() { let y = 2; }\n").is_ok());
    #[expect(clippy::expect_used, reason = "test needs absolute root")]
    let root = root.canonicalize().expect("canonicalize");
    let files = collect_source_files(
        std::slice::from_ref(&root),
        &WalkOptions::new(vec!["rs".to_owned()], Vec::new()),
    );
    #[expect(clippy::expect_used, reason = "test asserts walk ok")]
    let files = files.expect("walk");
    assert_eq!(files.len(), 2);
    (files, root)
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
