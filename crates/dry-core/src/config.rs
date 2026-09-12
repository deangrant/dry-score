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
    /// Both text and JSON (text on stdout, JSON on stderr path via runner).
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
    #[serde(default = "default_excludes")]
    pub exclude: Vec<String>,
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
            min_nodes: default_min_nodes(),
            min_lines: default_min_lines(),
            max_file_bytes: default_max_file_bytes(),
        }
    }
}

fn default_extensions() -> Vec<String> {
    vec!["rs".to_owned()]
}

fn default_excludes() -> Vec<String> {
    vec![
        "target".to_owned(),
        ".git".to_owned(),
        "fixtures".to_owned(),
        "tests".to_owned(),
    ]
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

/// Loads configuration from an explicit path.
///
/// # Errors
///
/// Returns [`ConfigError`] when the path is a symlink or otherwise not a
/// regular file, when the file cannot be read or parsed, or when
/// `gate.threshold` is outside `[0.0, 1.0]`.
pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    if !is_regular_file(path) {
        return Err(ConfigError::new(format!(
            "config path must be a regular file (symlinks are not followed): {}",
            path.display()
        )));
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| ConfigError::new(format!("failed to read {}: {err}", path.display())))?;
    let config: Config = toml::from_str(&raw)
        .map_err(|err| ConfigError::new(format!("failed to parse {}: {err}", path.display())))?;
    validate_threshold(config.gate.threshold)?;
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
mod tests {
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
                "tests".to_owned(),
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

    fn temp_dir(label: &str) -> PathBuf {
        // dry-rs:ignore. Per-module test temp-dir helper; shared shape is intentional.
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let base = std::env::temp_dir().join(format!("dry-rs-{label}-{stamp}"));
        assert!(fs::create_dir_all(&base).is_ok());
        base
    }
}
