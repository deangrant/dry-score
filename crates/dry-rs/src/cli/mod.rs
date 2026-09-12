//! Command-line argument parsing for `dry-rs`.

#[doc(inline)]
pub use dry_core::cli::{CliArgs, CliError, CliOptions, help_text};

/// Parses process arguments into [`CliArgs`] for the `dry-rs` binary.
///
/// # Errors
///
/// Returns [`CliError`] for unknown flags or invalid values.
pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<CliArgs, CliError> {
    dry_core::cli::parse_args(
        args,
        &CliOptions {
            bin_name: "dry-rs",
            force_extensions: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn wrapper_defaults_to_dry_rs_help() {
        let err = parse_args(["dry-rs".to_owned(), "--help".to_owned()]);
        assert!(err.is_err());
        #[expect(clippy::expect_used, reason = "test")]
        let err = err.expect_err("help");
        assert!(err.print_stdout);
        assert!(err.message.starts_with("dry-rs [PATH]..."));
        assert_eq!(
            parse_args(["dry-rs".to_owned()]).map(|a| a.paths).unwrap_or_default(),
            vec![PathBuf::from(".")]
        );
    }
}
