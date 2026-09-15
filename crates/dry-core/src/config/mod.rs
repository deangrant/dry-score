//! Configuration loading and discovery for `dry.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Output format selected by config or CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// Human-readable text.
    #[default]
    Text,
    /// JSON envelope.
    Json,
    /// Both text and JSON (text on stdout; JSON written to `--json-out`).
    Both,
}

impl OutputFormat {
    /// Parses a format token from CLI or config.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    /// Stable label for this format.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        // dry-rs:ignore. Per-enum vocabulary; shared match shape is intentional.
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Both => "both",
        }
    }
}

/// Gate knobs that control scoring and exit behavior.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GateConfig {
    /// Minimum Jaccard similarity to report.
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    /// Exit non-zero when findings exist.
    #[serde(default)]
    pub fail_on_findings: bool,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            threshold: default_threshold(),
            fail_on_findings: false,
        }
    }
}

const fn default_threshold() -> f64 {
    0.85
}

/// Output knobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    /// Report format.
    #[serde(default)]
    pub format: OutputFormat,
}

/// Filesystem walk knobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkConfig {
    /// Extensions without leading dots.
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    /// Path component names that cause a file or directory to be skipped.
    ///
    /// Unless [`Self::exclude_replace`] is true, user-supplied names are merged
    /// with [`default_excludes`].
    #[serde(default = "default_excludes")]
    pub exclude: Vec<String>,
    /// When true, [`Self::exclude`] replaces the built-in defaults instead of
    /// merging with them.
    #[serde(default)]
    pub exclude_replace: bool,
    /// Minimum structural nodes for a form.
    #[serde(default = "default_min_nodes")]
    pub min_nodes: u32,
    /// Minimum source lines for a form.
    #[serde(default = "default_min_lines")]
    pub min_lines: u32,
    /// Skip source files larger than this many bytes (not read or normalized).
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: u64,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            extensions: default_extensions(),
            exclude: default_excludes(),
            exclude_replace: false,
            min_nodes: default_min_nodes(),
            min_lines: default_min_lines(),
            max_file_bytes: default_max_file_bytes(),
        }
    }
}

fn default_extensions() -> Vec<String> {
    vec!["rs".to_owned()]
}

/// Built-in directory names skipped during walks when excludes merge with defaults.
#[must_use]
pub fn default_excludes() -> Vec<String> {
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
}

/// Merges `extra` into `base`, preserving order and dropping duplicate names.
#[must_use]
pub fn merge_excludes(base: &[String], extra: &[String]) -> Vec<String> {
    let mut out = base.to_vec();
    for name in extra {
        if !out.iter().any(|existing| existing == name) {
            out.push(name.clone());
        }
    }
    out
}

/// Applies default-exclude merge unless [`WalkConfig::exclude_replace`] is set.
pub fn resolve_walk_excludes(walk: &mut WalkConfig) {
    if !walk.exclude_replace {
        walk.exclude = merge_excludes(&default_excludes(), &walk.exclude);
    }
}

const fn default_min_nodes() -> u32 {
    10
}

const fn default_min_lines() -> u32 {
    3
}

const fn default_max_file_bytes() -> u64 {
    2_097_152
}

/// Root configuration loaded from `dry.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Config {
    /// Scoring and exit gate.
    #[serde(default)]
    pub gate: GateConfig,
    /// Output format.
    #[serde(default)]
    pub output: OutputConfig,
    /// Walk and size filters.
    #[serde(default)]
    pub walk: WalkConfig,
}

/// Errors from loading configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// Human-readable explanation.
    pub message: String,
}

impl ConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

/// Ensures `threshold` is a finite value in `[0.0, 1.0]`.
///
/// # Errors
///
/// Returns [`ConfigError`] when the value is NaN, infinite, or out of range.
pub fn validate_threshold(threshold: f64) -> Result<(), ConfigError> {
    if threshold.is_finite() && (0.0..=1.0).contains(&threshold) {
        Ok(())
    } else {
        Err(ConfigError::new(format!(
            "gate.threshold must be in [0.0, 1.0], got {threshold}"
        )))
    }
}

/// Ensures walk numeric knobs are at least `1`.
///
/// # Errors
///
/// Returns [`ConfigError`] when any of `min_nodes`, `min_lines`, or
/// `max_file_bytes` is zero.
pub fn validate_walk_numerics(walk: &WalkConfig) -> Result<(), ConfigError> {
    if walk.min_nodes < 1 {
        return Err(ConfigError::new(format!(
            "walk.min_nodes must be >= 1, got {}",
            walk.min_nodes
        )));
    }
    if walk.min_lines < 1 {
        return Err(ConfigError::new(format!(
            "walk.min_lines must be >= 1, got {}",
            walk.min_lines
        )));
    }
    if walk.max_file_bytes < 1 {
        return Err(ConfigError::new(format!(
            "walk.max_file_bytes must be >= 1, got {}",
            walk.max_file_bytes
        )));
    }
    Ok(())
}

/// Loads configuration from an explicit path.
///
/// # Errors
///
/// Returns [`ConfigError`] when the path is a symlink or otherwise not a
/// regular file, when the file cannot be read or parsed, when
/// `gate.threshold` is outside `[0.0, 1.0]`, or when walk numeric knobs are
/// below `1`.
pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    if !is_regular_file(path) {
        return Err(ConfigError::new(format!(
            "config path must be a regular file (symlinks are not followed): {}",
            path.display()
        )));
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| ConfigError::new(format!("failed to read {}: {err}", path.display())))?;
    parse_config_toml(path, &raw)
}

fn parse_config_toml(path: &Path, raw: &str) -> Result<Config, ConfigError> {
    let config: Config = toml::from_str(raw)
        .map_err(|err| ConfigError::new(format!("failed to parse {}: {err}", path.display())))?;
    validate_threshold(config.gate.threshold)?;
    validate_walk_numerics(&config.walk)?;
    let mut config = config;
    resolve_walk_excludes(&mut config.walk);
    Ok(config)
}

/// Walks upward from `start` looking for `dry.toml`.
///
/// Symlinked `dry.toml` files are ignored so config discovery matches the
/// walker policy of not following symlinks.
#[must_use]
pub fn discover_config(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        let candidate = dir.join("dry.toml");
        if is_regular_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_file())
}

#[cfg(test)]
mod tests;
