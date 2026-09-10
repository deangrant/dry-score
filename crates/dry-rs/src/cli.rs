//! Command-line argument parsing for `dry-rs`.

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dry_core::{Config, OutputFormat, discover_config, load_config, validate_threshold};

/// Parsed CLI invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct CliArgs {
    /// Roots to analyze.
    pub paths: Vec<PathBuf>,
    /// Effective configuration after CLI overlays.
    pub config: Config,
    /// Optional path to write JSON when format is `both`.
    pub json_out: Option<PathBuf>,
}

/// CLI parse / usage errors.
#[derive(Debug, Clone, PartialEq)]
pub struct CliError {
    /// Human-readable explanation.
    pub message: String,
    /// Suggested process exit code.
    pub exit: ExitCode,
    /// When true, callers should print [`Self::message`] to stdout.
    pub print_stdout: bool,
}

impl CliError {
    /// Builds a usage/config error with exit code 2.
    #[must_use]
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit: ExitCode::from(2),
            print_stdout: false,
        }
    }

    /// Builds a help response with exit code 0 (print to stdout).
    #[must_use]
    pub fn help(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit: ExitCode::SUCCESS,
            print_stdout: true,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

/// Parses process arguments into [`CliArgs`].
///
/// # Errors
///
/// Returns [`CliError`] for unknown flags or invalid values.
pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<CliArgs, CliError> {
    let mut args = args.into_iter();
    let _exe = args.next();
    let mut raw = RawFlags::default();
    while let Some(arg) = args.next() {
        apply_arg(&arg, &mut args, &mut raw)?;
    }
    finish_args(raw)
}

#[derive(Debug, Default)]
struct RawFlags {
    paths: Vec<PathBuf>,
    config_path: Option<PathBuf>,
    threshold: Option<f64>,
    format: Option<OutputFormat>,
    min_nodes: Option<u32>,
    min_lines: Option<u32>,
    fail_on: Option<bool>,
    json_out: Option<PathBuf>,
}

fn apply_arg(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    if let Some(result) = try_flag_arg(arg, args, raw) {
        return result;
    }
    raw.paths.push(PathBuf::from(arg));
    Ok(())
}

fn try_flag_arg(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    if arg == "--help" || arg == "-h" {
        return Some(Err(CliError::help(help_text())));
    }
    try_apply_valued(arg, args, raw)
        .or_else(|| try_apply_switch(arg, raw))
        .or_else(|| unknown_flag(arg))
}

fn unknown_flag(arg: &str) -> Option<Result<(), CliError>> {
    if arg.starts_with('-') {
        Some(Err(CliError::usage(format!("unknown flag: {arg}"))))
    } else {
        None
    }
}

fn try_apply_valued(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    try_apply_configish(arg, args, raw).or_else(|| try_apply_outputish(arg, args, raw))
}

fn try_apply_configish(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    try_apply_config_threshold(arg, args, raw).or_else(|| try_apply_mins(arg, args, raw))
}

fn try_apply_config_threshold(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    match arg {
        "--config" => Some(apply_config(args, raw)),
        "--threshold" => Some(apply_threshold(args, raw)),
        _ => None,
    }
}

fn try_apply_mins(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    match arg {
        "--min-nodes" => Some(apply_min_nodes(args, raw)),
        "--min-lines" => Some(apply_min_lines(args, raw)),
        _ => None,
    }
}

fn try_apply_outputish(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    match arg {
        "--format" => Some(apply_format(args, raw)),
        "--json-out" => Some(apply_json_out(args, raw)),
        _ => None,
    }
}

fn try_apply_switch(arg: &str, raw: &mut RawFlags) -> Option<Result<(), CliError>> {
    match arg {
        "--fail-on-findings" => {
            raw.fail_on = Some(true);
            Some(Ok(()))
        }
        "--no-fail-on-findings" => {
            raw.fail_on = Some(false);
            Some(Ok(()))
        }
        _ => None,
    }
}

fn apply_config(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    raw.config_path = Some(PathBuf::from(require_value(args, "--config")?));
    Ok(())
}

fn apply_threshold(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    raw.threshold = Some(parse_value(&require_value(args, "--threshold")?, "float")?);
    Ok(())
}

fn apply_format(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    let value = require_value(args, "--format")?;
    raw.format =
        Some(OutputFormat::parse(&value).ok_or_else(|| CliError::usage("invalid --format"))?);
    Ok(())
}

fn apply_min_nodes(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    raw.min_nodes = Some(parse_value(
        &require_value(args, "--min-nodes")?,
        "integer",
    )?);
    Ok(())
}

fn apply_min_lines(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    raw.min_lines = Some(parse_value(
        &require_value(args, "--min-lines")?,
        "integer",
    )?);
    Ok(())
}

fn apply_json_out(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    raw.json_out = Some(PathBuf::from(require_value(args, "--json-out")?));
    Ok(())
}

fn finish_args(mut raw: RawFlags) -> Result<CliArgs, CliError> {
    if raw.paths.is_empty() {
        raw.paths.push(PathBuf::from("."));
    }
    let mut config = load_effective_config(raw.config_path.as_deref())?;
    overlay_cli_onto_config(&raw, &mut config);
    validate_threshold(config.gate.threshold).map_err(|err| CliError::usage(err.to_string()))?;
    Ok(CliArgs {
        paths: raw.paths,
        config,
        json_out: raw.json_out,
    })
}

const fn overlay_cli_onto_config(raw: &RawFlags, config: &mut Config) {
    overlay_gate_flags(raw, config);
    overlay_output_flags(raw, config);
    overlay_walk_flags(raw, config);
}

const fn overlay_gate_flags(raw: &RawFlags, config: &mut Config) {
    if let Some(threshold) = raw.threshold {
        config.gate.threshold = threshold;
    }
    if let Some(fail_on) = raw.fail_on {
        config.gate.fail_on_findings = fail_on;
    }
}

const fn overlay_output_flags(raw: &RawFlags, config: &mut Config) {
    if let Some(format) = raw.format {
        config.output.format = format;
    }
}

const fn overlay_walk_flags(raw: &RawFlags, config: &mut Config) {
    if let Some(min_nodes) = raw.min_nodes {
        config.walk.min_nodes = min_nodes;
    }
    if let Some(min_lines) = raw.min_lines {
        config.walk.min_lines = min_lines;
    }
}

fn require_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, CliError> {
    args.next().ok_or_else(|| CliError::usage(format!("missing value for {flag}")))
}

fn load_effective_config(explicit: Option<&Path>) -> Result<Config, CliError> {
    if let Some(path) = explicit {
        return load_config(path).map_err(|err| CliError::usage(err.to_string()));
    }
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Some(found) = discover_config(&cwd) {
        return load_config(&found).map_err(|err| CliError::usage(err.to_string()));
    }
    Ok(Config::default())
}

fn parse_value<T: std::str::FromStr>(raw: &str, kind: &str) -> Result<T, CliError> {
    raw.parse::<T>().map_err(|_| CliError::usage(format!("invalid {kind}: {raw}")))
}

fn help_text() -> String {
    "dry-rs [PATH]... [options]\n\n\
     --config PATH\n\
     --threshold FLOAT\n\
     --format text|json|both\n\
     --min-nodes N\n\
     --min-lines N\n\
     --fail-on-findings\n\
     --no-fail-on-findings\n\
     --json-out PATH\n\
     --help\n"
        .to_owned()
}

#[cfg(test)]
mod tests {
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
    fn min_lines_parse_errors() {
        assert!(parse_args(args(&["--min-lines"])).is_err());
        assert!(parse_args(args(&["--min-lines", "x"])).is_err());
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
}
