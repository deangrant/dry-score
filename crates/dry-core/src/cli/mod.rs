//! Language-agnostic command-line argument parsing.

mod apply;

#[cfg(test)]
mod tests;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::{
    Config, OutputFormat, discover_config, load_config, validate_threshold, validate_walk_numerics,
};

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
    /// Binary name used for help text and report headers.
    pub bin_name: &'static str,
}

/// Options that configure CLI parsing for a language-adapter binary.
#[derive(Debug, Clone)]
pub struct CliOptions {
    /// Binary name shown in `--help` text.
    pub bin_name: &'static str,
    /// When set, replaces `walk.extensions` after config load and CLI overlays.
    pub force_extensions: Option<Vec<String>>,
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
pub fn parse_args(
    args: impl IntoIterator<Item = String>,
    options: &CliOptions,
) -> Result<CliArgs, CliError> {
    let mut args = args.into_iter();
    let _exe = args.next();
    let mut raw = RawFlags::default();
    while let Some(arg) = args.next() {
        apply_arg(&arg, &mut args, &mut raw, options)?;
    }
    finish_args(raw, options)
}

#[derive(Debug, Default)]
struct RawFlags {
    paths: Vec<PathBuf>,
    config_path: Option<PathBuf>,
    threshold: Option<f64>,
    format: Option<OutputFormat>,
    min_nodes: Option<u32>,
    min_lines: Option<u32>,
    max_file_bytes: Option<u64>,
    extensions: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    exclude_only: Option<Vec<String>>,
    fail_on: Option<bool>,
    json_out: Option<PathBuf>,
}

fn finish_args(mut raw: RawFlags, options: &CliOptions) -> Result<CliArgs, CliError> {
    if raw.paths.is_empty() {
        raw.paths.push(PathBuf::from("."));
    }
    let mut config = load_effective_config(raw.config_path.as_deref(), &raw.paths[0])?;
    overlay_cli_onto_config(&raw, &mut config);
    reject_cli_extensions_when_forced(&raw, options)?;
    apply_forced_extensions(options, &mut config);
    validate_cli_config(&config)?;
    Ok(CliArgs {
        paths: raw.paths,
        config,
        json_out: raw.json_out,
        bin_name: options.bin_name,
    })
}

fn reject_cli_extensions_when_forced(raw: &RawFlags, options: &CliOptions) -> Result<(), CliError> {
    if options.force_extensions.is_none() || raw.extensions.is_none() {
        return Ok(());
    }
    let fixed = options.force_extensions.as_ref().map_or(String::new(), |exts| exts.join(", "));
    Err(CliError::usage(format!(
        "--extensions is not supported for {}; extensions are fixed to [{fixed}]",
        options.bin_name
    )))
}

fn apply_forced_extensions(options: &CliOptions, config: &mut Config) {
    if let Some(extensions) = &options.force_extensions {
        config.walk.extensions.clone_from(extensions);
    }
}

fn validate_cli_config(config: &Config) -> Result<(), CliError> {
    validate_threshold(config.gate.threshold).map_err(|err| CliError::usage(err.to_string()))?;
    validate_walk_numerics(&config.walk).map_err(|err| CliError::usage(err.to_string()))
}

fn load_effective_config(explicit: Option<&Path>, first_root: &Path) -> Result<Config, CliError> {
    if let Some(path) = explicit {
        return load_config(path).map_err(|err| CliError::usage(err.to_string()));
    }
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Some(found) = discover_config(&cwd) {
        return load_config(&found).map_err(|err| CliError::usage(err.to_string()));
    }
    if let Some(root) = analysis_root_for_discovery(&cwd, first_root)
        && let Some(found) = discover_config(&root)
    {
        return load_config(&found).map_err(|err| CliError::usage(err.to_string()));
    }
    Ok(Config::default())
}

fn analysis_root_for_discovery(cwd: &Path, first_root: &Path) -> Option<PathBuf> {
    if first_root.as_os_str() == "." {
        return None;
    }
    let root = if first_root.is_absolute() {
        first_root.to_path_buf()
    } else {
        cwd.join(first_root)
    };
    if paths_equivalent(cwd, &root) {
        None
    } else {
        Some(root)
    }
}

fn paths_equivalent(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Builds the `--help` text for the given CLI options.
#[must_use]
pub fn help_text(options: &CliOptions) -> String {
    let extensions_line = options.force_extensions.as_ref().map_or_else(
        || "         --extensions EXT[,EXT]...\n".to_owned(),
        |exts| {
            format!(
                "         (extensions fixed to {}; --extensions unsupported)\n",
                exts.join(", ")
            )
        },
    );
    format!(
        "{bin} [PATH]... [options]\n\n\
         --config PATH\n\
         --threshold FLOAT\n\
         --format text|json|both\n\
         --min-nodes N\n\
         --min-lines N\n\
         --max-file-bytes N\n\
         {extensions_line}\
         --exclude NAME[,NAME]...   (merge with defaults)\n\
         --exclude-only NAME[,NAME]...   (replace defaults)\n\
         --fail-on-findings\n\
         --no-fail-on-findings\n\
         --json-out PATH   (required with --format both; overwrites PATH)\n\
         --help\n",
        bin = options.bin_name,
    )
}
