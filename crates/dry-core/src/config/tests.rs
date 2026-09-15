use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn format_as_str_covers_variants() {
    // dry-rs:ignore. OutputFormat vocabulary test; shared assert shape is intentional.
    assert_eq!(OutputFormat::Text.as_str(), "text");
    assert_eq!(OutputFormat::Json.as_str(), "json");
    assert_eq!(OutputFormat::Both.as_str(), "both");
}

#[test]
fn default_threshold_is_review_floor() {
    assert!((Config::default().gate.threshold - 0.85).abs() < f64::EPSILON);
}

#[test]
fn default_excludes_match_example() {
    assert_eq!(
        WalkConfig::default().exclude,
        vec![
            "target".to_owned(),
            ".git".to_owned(),
            "fixtures".to_owned(),
            "node_modules".to_owned(),
            "vendor".to_owned(),
            ".venv".to_owned(),
            "venv".to_owned(),
            "dist".to_owned(),
            "__pycache__".to_owned(),
        ]
    );
}

#[test]
fn parse_format_tokens() {
    assert_eq!(OutputFormat::parse("text"), Some(OutputFormat::Text));
    assert_eq!(OutputFormat::parse("json"), Some(OutputFormat::Json));
    assert_eq!(OutputFormat::parse("both"), Some(OutputFormat::Both));
    assert_eq!(OutputFormat::parse("nope"), None);
}

#[test]
fn load_config_reads_toml() {
    let dir = temp_dir("cfg-ok");
    let path = dir.join("dry.toml");
    assert!(fs::write(&path, "[gate]\nthreshold = 0.9\n").is_ok());
    let cfg = load_config(&path);
    assert!(cfg.is_ok());
    #[expect(clippy::expect_used, reason = "test asserts load succeeded")]
    let cfg = cfg.expect("ok");
    assert!((cfg.gate.threshold - 0.9).abs() < f64::EPSILON);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_config_missing_file_errors() {
    let err = load_config(Path::new("/no/such/dry.toml"));
    assert!(err.is_err());
    #[expect(clippy::expect_used, reason = "test asserts error path")]
    let err = err.expect_err("missing");
    assert!(!err.to_string().is_empty());
}

#[test]
fn load_config_invalid_toml_errors() {
    let dir = temp_dir("cfg-bad");
    let path = dir.join("dry.toml");
    assert!(fs::write(&path, "[[[not toml").is_ok());
    assert!(load_config(&path).is_err());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn validate_threshold_bounds() {
    assert!(validate_threshold(0.0).is_ok());
    assert!(validate_threshold(1.0).is_ok());
    assert!(validate_threshold(0.85).is_ok());
    assert!(validate_threshold(-0.1).is_err());
    assert!(validate_threshold(1.1).is_err());
    assert!(validate_threshold(f64::NAN).is_err());
    assert!(validate_threshold(f64::INFINITY).is_err());
}

#[test]
fn validate_walk_numerics_requires_positive() {
    let mut walk = WalkConfig::default();
    assert!(validate_walk_numerics(&walk).is_ok());
    walk.min_nodes = 0;
    assert!(validate_walk_numerics(&walk).is_err());
    walk = WalkConfig::default();
    walk.min_lines = 0;
    assert!(validate_walk_numerics(&walk).is_err());
    walk = WalkConfig::default();
    walk.max_file_bytes = 0;
    assert!(validate_walk_numerics(&walk).is_err());
}

#[test]
fn load_config_rejects_out_of_range_threshold() {
    let dir = temp_dir("cfg-threshold");
    let low = dir.join("low.toml");
    let high = dir.join("high.toml");
    assert!(fs::write(&low, "[gate]\nthreshold = -0.1\n").is_ok());
    assert!(fs::write(&high, "[gate]\nthreshold = 1.1\n").is_ok());
    assert!(load_config(&low).is_err());
    assert!(load_config(&high).is_err());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_config_rejects_zero_walk_numerics() {
    let dir = temp_dir("cfg-walk-zero");
    let nodes = dir.join("nodes.toml");
    let lines = dir.join("lines.toml");
    let bytes = dir.join("bytes.toml");
    assert!(fs::write(&nodes, "[walk]\nmin_nodes = 0\n").is_ok());
    assert!(fs::write(&lines, "[walk]\nmin_lines = 0\n").is_ok());
    assert!(fs::write(&bytes, "[walk]\nmax_file_bytes = 0\n").is_ok());
    assert!(load_config(&nodes).is_err());
    assert!(load_config(&lines).is_err());
    assert!(load_config(&bytes).is_err());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn discover_config_walks_upward() {
    let dir = temp_dir("discover");
    let nested = dir.join("a/b");
    assert!(fs::create_dir_all(&nested).is_ok());
    assert!(fs::write(dir.join("dry.toml"), "[gate]\n").is_ok());
    let found = discover_config(&nested);
    assert!(found.is_some());
    assert!(discover_config(Path::new("/tmp/dry-rs-no-config-ancestor-xyz")).is_none());
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn discover_and_load_skip_symlink_dry_toml() {
    let dir = temp_dir("cfg-symlink");
    let target = dir.join("outside.toml");
    assert!(fs::write(&target, "[gate]\nthreshold = 0.9\n").is_ok());
    let link = dir.join("dry.toml");
    assert!(std::os::unix::fs::symlink(&target, &link).is_ok());
    assert!(discover_config(&dir).is_none());
    let err = load_config(&link);
    assert!(err.is_err());
    #[expect(clippy::expect_used, reason = "test asserts error path")]
    let err = err.expect_err("symlink");
    assert!(err.to_string().contains("symlinks"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_config_merges_exclude_with_defaults() {
    let dir = temp_dir("cfg-exclude-merge");
    let path = dir.join("dry.toml");
    assert!(fs::write(&path, "[walk]\nexclude = [\"tests\"]\n").is_ok());
    let cfg = load_config(&path);
    assert!(cfg.is_ok());
    #[expect(clippy::expect_used, reason = "test asserts load succeeded")]
    let cfg = cfg.expect("ok");
    assert!(cfg.walk.exclude.iter().any(|n| n == "tests"));
    assert!(cfg.walk.exclude.iter().any(|n| n == ".git"));
    assert!(cfg.walk.exclude.iter().any(|n| n == "node_modules"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_config_exclude_replace_skips_merge() {
    let dir = temp_dir("cfg-exclude-replace");
    let path = dir.join("dry.toml");
    assert!(
        fs::write(
            &path,
            "[walk]\nexclude_replace = true\nexclude = [\"tests\"]\n"
        )
        .is_ok()
    );
    let cfg = load_config(&path);
    assert!(cfg.is_ok());
    #[expect(clippy::expect_used, reason = "test asserts load succeeded")]
    let cfg = cfg.expect("ok");
    assert_eq!(cfg.walk.exclude, vec!["tests".to_owned()]);
    let _ = fs::remove_dir_all(dir);
}

fn temp_dir(label: &str) -> PathBuf {
    // dry-rs:ignore. Per-module test temp-dir helper; shared shape is intentional.
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let base = std::env::temp_dir().join(format!("dry-rs-{label}-{stamp}"));
    assert!(fs::create_dir_all(&base).is_ok());
    base
}
