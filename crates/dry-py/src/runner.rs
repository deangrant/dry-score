//! Binary orchestration for `dry-py`.

use std::process::ExitCode;

use dry_core::{CliError, CliOptions, parse_args, run_analysis};

use crate::normalize::PyNormalizer;

/// Parses process args and runs analysis.
///
/// # Errors
///
/// Returns [`CliError`] for usage failures or analysis errors.
pub fn run_from_env() -> Result<ExitCode, CliError> {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    let args = parse_args(
        std::env::args(),
        &CliOptions {
            bin_name: "dry-py",
            force_extensions: Some(vec!["py".to_owned()]),
        },
    )?;
    let normalizer = PyNormalizer::new(args.config.walk.min_nodes, args.config.walk.min_lines);
    run_analysis(&args, &normalizer)
}
