use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn args(items: &[&str]) -> Vec<String> {
    std::iter::once("dry-rs".to_owned())
        .chain(items.iter().map(|s| (*s).to_owned()))
        .collect()
}

#[test]
fn parse_defaults_and_flags() {
    let parsed = parse_args(args(&["src", "--threshold", "0.9", "--format", "json"]));
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert_eq!(parsed.paths, vec![PathBuf::from("src")]);
    assert!((parsed.config.gate.threshold - 0.9).abs() < f64::EPSILON);
    assert_eq!(parsed.config.output.format, OutputFormat::Json);
}

#[test]
fn parse_switches() {
    let parsed = parse_args(args(&[
        "--fail-on-findings",
        "--no-fail-on-findings",
        "--min-nodes",
        "12",
        "--min-lines",
        "5",
        "--json-out",
        "out.json",
    ]));
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
    assert!(parse_args(args(&["--unknown"])).is_err());
    assert!(parse_args(args(&["--threshold"])).is_err());
    assert!(parse_args(args(&["--format", "nope"])).is_err());
    assert!(parse_args(args(&["--min-nodes", "x"])).is_err());
    assert!(parse_args(args(&["--min-lines"])).is_err());
    assert!(parse_args(args(&["--min-lines", "x"])).is_err());
}

#[test]
fn help_exits_success_via_stdout() {
    for flag in ["--help", "-h"] {
        let err = parse_args(args(&[flag]));
        assert!(err.is_err());
        #[expect(clippy::expect_used, reason = "test")]
        let err = err.expect_err("help");
        assert!(err.print_stdout);
        assert!(err.message.contains("--threshold"));
    }
}

#[test]
fn threshold_bounds() {
    assert!(parse_args(args(&["--threshold", "0.0"])).is_ok());
    assert!(parse_args(args(&["--threshold", "1.0"])).is_ok());
    assert!(parse_args(args(&["--threshold", "-0.1"])).is_err());
    assert!(parse_args(args(&["--threshold", "1.1"])).is_err());
}

#[test]
fn default_path_is_dot() {
    let parsed = parse_args(args(&[]));
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert_eq!(parsed.paths, vec![PathBuf::from(".")]);
    assert!(!CliError::usage("x").to_string().is_empty());
    assert!(!help_text().is_empty());
}

#[test]
#[expect(
    clippy::cognitive_complexity,
    reason = "config discovery matrix spans explicit/cwd/default paths"
)]
fn explicit_config_path() {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("dry-rs-cli-{stamp}"));
    assert!(fs::create_dir_all(&dir).is_ok());
    let cfg = dir.join("custom.toml");
    assert!(fs::write(&cfg, "[gate]\nthreshold = 0.77\n").is_ok());
    let cfg_s = cfg.to_string_lossy().into_owned();
    let parsed = parse_args(args(&["--config", &cfg_s]));
    assert!(parsed.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let parsed = parsed.expect("ok");
    assert!((parsed.config.gate.threshold - 0.77).abs() < f64::EPSILON);
    assert!(parse_args(args(&["--config", "/no/such.toml"])).is_err());
    let discovered = load_effective_config(None);
    assert!(discovered.is_ok());
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let bare = std::env::temp_dir().join(format!("dry-rs-cli-bare-{stamp}"));
    assert!(fs::create_dir_all(&bare).is_ok());
    let previous = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    assert!(env::set_current_dir(&bare).is_ok());
    let defaulted = load_effective_config(None);
    assert!(env::set_current_dir(previous).is_ok());
    assert!(defaulted.is_ok());
    #[expect(clippy::expect_used, reason = "test")]
    let defaulted = defaulted.expect("ok");
    assert!((defaulted.gate.threshold - Config::default().gate.threshold).abs() < f64::EPSILON);
    let _ = fs::remove_dir_all(bare);
    let _ = fs::remove_dir_all(dir);
}
