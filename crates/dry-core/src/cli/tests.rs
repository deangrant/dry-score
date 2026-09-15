use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn opts() -> CliOptions {
    CliOptions {
        bin_name: "dry-core-test",
        force_extensions: None,
    }
}

fn args(items: &[&str]) -> Vec<String> {
    std::iter::once("dry-core-test".to_owned())
        .chain(items.iter().map(|s| (*s).to_owned()))
        .collect()
}

fn parse(items: &[&str]) -> Result<CliArgs, CliError> {
    parse_args(args(items), &opts())
}

#[test]
fn parse_defaults_and_flags() {
    let parsed = parse(&["src", "--threshold", "0.9", "--format", "json"]);
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert_eq!(parsed.paths, vec![PathBuf::from("src")]);
    assert!((parsed.config.gate.threshold - 0.9).abs() < f64::EPSILON);
    assert_eq!(parsed.config.output.format, OutputFormat::Json);
}

#[test]
fn parse_switches() {
    let parsed = parse(&[
        "--fail-on-findings",
        "--no-fail-on-findings",
        "--min-nodes",
        "12",
        "--min-lines",
        "5",
        "--json-out",
        "out.json",
    ]);
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert!(!parsed.config.gate.fail_on_findings);
    assert_eq!(parsed.config.walk.min_nodes, 12);
    assert_eq!(parsed.config.walk.min_lines, 5);
    assert_eq!(parsed.json_out.as_deref(), Some(Path::new("out.json")));
}

#[test]
fn parse_errors() {
    assert!(parse(&["--unknown"]).is_err());
    assert!(parse(&["--threshold"]).is_err());
    assert!(parse(&["--format", "nope"]).is_err());
    assert!(parse(&["--min-nodes", "x"]).is_err());
    assert!(parse(&["--min-lines"]).is_err());
    assert!(parse(&["--min-lines", "x"]).is_err());
}

#[test]
fn walk_flag_value_required() {
    assert!(parse(&["--extensions"]).is_err());
    assert!(parse(&["--exclude"]).is_err());
    assert!(parse(&["--exclude-only"]).is_err());
}

#[test]
fn help_exits_success_via_stdout() {
    for flag in ["--help", "-h"] {
        let err = parse(&[flag]);
        assert!(err.is_err());
        #[expect(clippy::expect_used, reason = "test")]
        let err = err.expect_err("help");
        assert!(err.print_stdout);
        assert!(err.message.contains("--threshold"));
        assert!(err.message.contains("--exclude-only"));
        assert!(err.message.starts_with("dry-core-test [PATH]..."));
    }
}

#[test]
fn threshold_bounds() {
    assert!(parse(&["--threshold", "0.0"]).is_ok());
    assert!(parse(&["--threshold", "1.0"]).is_ok());
    assert!(parse(&["--threshold", "-0.1"]).is_err());
    assert!(parse(&["--threshold", "1.1"]).is_err());
}

#[test]
fn default_path_is_dot() {
    let parsed = parse(&[]);
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert_eq!(parsed.paths, vec![PathBuf::from(".")]);
    assert!(!CliError::usage("x").to_string().is_empty());
    assert!(!help_text("dry-core-test").is_empty());
}

#[test]
fn parse_extensions_and_exclude_merge_with_defaults() {
    let parsed = parse(&["--extensions", "rs, go", "--exclude", "tests"]);
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert_eq!(
        parsed.config.walk.extensions,
        vec!["rs".to_owned(), "go".to_owned()]
    );
    assert!(parsed.config.walk.exclude.iter().any(|n| n == "tests"));
    assert!(parsed.config.walk.exclude.iter().any(|n| n == ".git"));
    assert!(parsed.config.walk.exclude.iter().any(|n| n == "node_modules"));
}

#[test]
fn parse_exclude_only_replaces_defaults() {
    let parsed = parse(&["--exclude-only", "target,vendor"]);
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert_eq!(
        parsed.config.walk.exclude,
        vec!["target".to_owned(), "vendor".to_owned()]
    );
    assert!(parsed.config.walk.exclude_replace);
}

#[test]
fn force_extensions_overrides_cli_extensions() {
    let parsed = parse_args(
        args(&["--extensions", "rs"]),
        &CliOptions {
            bin_name: "dry-go",
            force_extensions: Some(vec!["go".to_owned()]),
        },
    );
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert_eq!(parsed.config.walk.extensions, vec!["go".to_owned()]);
}

#[test]
fn force_extensions_overrides_config() {
    let parsed = parse_args(
        args(&[]),
        &CliOptions {
            bin_name: "dry-go",
            force_extensions: Some(vec!["go".to_owned()]),
        },
    );
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert_eq!(parsed.config.walk.extensions, vec!["go".to_owned()]);
}

#[test]
#[expect(
    clippy::cognitive_complexity,
    reason = "config discovery matrix spans explicit/cwd/root/default paths"
)]
fn explicit_config_path() {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("dry-core-cli-{stamp}"));
    assert!(fs::create_dir_all(&dir).is_ok());
    let cfg = dir.join("custom.toml");
    assert!(fs::write(&cfg, "[gate]\nthreshold = 0.77\n").is_ok());
    let cfg_s = cfg.to_string_lossy().into_owned();
    let parsed = parse(&["--config", &cfg_s]);
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert!((parsed.config.gate.threshold - 0.77).abs() < f64::EPSILON);
    assert!(parse(&["--config", "/no/such.toml"]).is_err());
    let discovered = load_effective_config(None, Path::new("."));
    assert!(discovered.is_ok());
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let bare = std::env::temp_dir().join(format!("dry-core-cli-bare-{stamp}"));
    assert!(fs::create_dir_all(&bare).is_ok());
    let previous = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    assert!(env::set_current_dir(&bare).is_ok());
    let defaulted = load_effective_config(None, Path::new("."));
    assert!(env::set_current_dir(&previous).is_ok());
    assert!(defaulted.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let defaulted = defaulted.expect("ok");
    assert!((defaulted.gate.threshold - Config::default().gate.threshold).abs() < f64::EPSILON);
    let _ = fs::remove_dir_all(bare);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn discovers_config_from_analysis_root_when_cwd_has_none() {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let bare = std::env::temp_dir().join(format!("dry-core-cli-cwd-{stamp}"));
    let root = std::env::temp_dir().join(format!("dry-core-cli-root-{stamp}"));
    assert!(fs::create_dir_all(&bare).is_ok());
    assert!(fs::create_dir_all(&root).is_ok());
    assert!(fs::write(root.join("dry.toml"), "[gate]\nthreshold = 0.71\n").is_ok());
    let previous = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    assert!(env::set_current_dir(&bare).is_ok());
    let root_s = root.to_string_lossy().into_owned();
    let parsed = parse(&[&root_s]);
    assert!(env::set_current_dir(previous).is_ok());
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert!((parsed.config.gate.threshold - 0.71).abs() < f64::EPSILON);
    let _ = fs::remove_dir_all(bare);
    let _ = fs::remove_dir_all(root);
}
