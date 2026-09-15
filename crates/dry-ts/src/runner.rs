//! Binary orchestration for `dry-ts`.

use std::process::ExitCode;

use dry_core::{CliError, CliOptions, parse_args, run_analysis};

use crate::normalize::TsNormalizer;

/// Parses process args and runs analysis.
///
/// # Errors
///
/// Returns [`CliError`] for usage failures or analysis errors.
pub fn run_from_env() -> Result<ExitCode, CliError> {
    let args = parse_args(
        std::env::args(),
        &CliOptions {
            bin_name: "dry-ts",
            force_extensions: Some(vec![
                "ts".to_owned(),
                "tsx".to_owned(),
                "mts".to_owned(),
                "cts".to_owned(),
            ]),
        },
    )?;
    let normalizer = TsNormalizer::new(args.config.walk.min_nodes, args.config.walk.min_lines);
    run_analysis(&args, &normalizer)
}
