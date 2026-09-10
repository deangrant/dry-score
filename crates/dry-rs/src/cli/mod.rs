//! Command-line argument parsing for `dry-rs`.

mod apply;

#[cfg(test)]
mod tests;

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dry_core::{Config, OutputFormat, discover_config, load_config, validate_threshold};

use apply::{apply_arg, overlay_cli_onto_config};

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
