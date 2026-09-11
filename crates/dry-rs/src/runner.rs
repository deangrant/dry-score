//! Binary orchestration for `dry-rs`.

use std::process::ExitCode;

use dry_core::cli::{CliArgs, CliError};
use dry_core::runner::run_analysis;

use crate::cli::parse_args;
use crate::normalize::RustNormalizer;

#[doc(inline)]
pub use dry_core::runner::{emit_report, exit_for_findings, print_err, print_out};

/// Parses process args and runs analysis.
///
/// # Errors
///
/// Returns [`CliError`] for usage failures or analysis errors.
pub fn run_from_env() -> Result<ExitCode, CliError> {
    run(&parse_args(std::env::args())?)
}

/// Runs analysis for already-parsed CLI arguments.
///
/// # Errors
///
/// Returns [`CliError`] when analysis or report emission fails.
pub fn run(args: &CliArgs) -> Result<ExitCode, CliError> {
    let normalizer = RustNormalizer::new(args.config.walk.min_nodes, args.config.walk.min_lines);
    run_analysis(args, &normalizer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dry_core::{Config, OutputFormat, Report, ReportSummary};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project() -> std::path::PathBuf {
        // dry-rs:ignore. Temp-dir harness shape differs from walk by design.
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let base = std::env::temp_dir().join(format!("dry-rs-runner-{stamp}"));
        assert!(fs::create_dir_all(base.join("src")).is_ok());
        assert!(
            fs::write(
                base.join("src/lib.rs"),
                concat!(
                    "fn one() {\n    let a = 1;\n    let b = a + 1;\n    let c = b + 1;\n}\n",
                    "fn two() {\n    let a = 1;\n    let b = a + 1;\n    let c = b + 1;\n}\n",
                ),
            )
            .is_ok()
        );
        base
    }

    #[test]
    fn run_and_emit_formats() {
        let base = temp_project();
        let mut config = Config::default();
        config.walk.min_nodes = 5;
        config.walk.min_lines = 3;
        config.output.format = OutputFormat::Text;
        let args = CliArgs {
            paths: vec![base.clone()],
            config,
            json_out: Some(base.join("from-run.json")),
        };
        assert!(run(&args).is_ok());

        let report = Report::new(0.85, Vec::new(), ReportSummary::default(), Vec::new());
        assert!(emit_report(&report, OutputFormat::Text, None).is_ok());
        assert!(emit_report(&report, OutputFormat::Json, None).is_ok());
        assert!(emit_report(&report, OutputFormat::Both, None).is_ok());
        let out = base.join("report.json");
        assert!(emit_report(&report, OutputFormat::Both, Some(&out)).is_ok());
        assert!(exit_for_findings(true, false) == ExitCode::from(1));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn run_emit_report_error_propagates() {
        let base = temp_project();
        let mut config = Config::default();
        config.walk.min_nodes = 5;
        config.walk.min_lines = 3;
        config.output.format = OutputFormat::Both;
        let args = CliArgs {
            paths: vec![base.clone()],
            config,
            json_out: Some(base.join("missing").join("out.json")),
        };
        assert!(run(&args).is_err());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn run_fail_on_findings() {
        let base = temp_project();
        let mut config = Config::default();
        config.walk.min_nodes = 5;
        config.walk.min_lines = 3;
        config.gate.threshold = 0.5;
        config.gate.fail_on_findings = true;
        let args = CliArgs {
            paths: vec![base.clone()],
            config,
            json_out: None,
        };
        let code = run(&args);
        assert!(code.is_ok());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn run_analyze_error_on_missing_root() {
        let args = CliArgs {
            paths: vec![std::path::PathBuf::from("/no/such/dry-rs-runner-root")],
            config: Config::default(),
            json_out: None,
        };
        assert!(run(&args).is_err());
    }
}
