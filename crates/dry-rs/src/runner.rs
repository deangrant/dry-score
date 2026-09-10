//! Binary orchestration for `dry-rs`.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use dry_core::{OutputFormat, Report, analyze, render_json, render_text};

use crate::cli::{CliArgs, CliError, parse_args};
use crate::normalize::RustNormalizer;

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
    let result = analyze(&args.paths, &args.config, &normalizer).map_err(|message| CliError {
        message,
        exit: ExitCode::from(2),
    })?;
    emit_report(
        &result.report,
        args.config.output.format,
        args.json_out.as_deref(),
    )?;
    Ok(exit_for_findings(
        args.config.gate.fail_on_findings,
        result.report.findings.is_empty(),
    ))
}

fn exit_for_findings(fail_on: bool, empty: bool) -> ExitCode {
    if fail_on && !empty {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Writes the report according to `format`.
///
/// # Errors
///
/// Returns [`CliError`] when JSON rendering or file writes fail.
pub fn emit_report(
    report: &Report,
    format: OutputFormat,
    json_out: Option<&Path>,
) -> Result<(), CliError> {
    match format {
        OutputFormat::Text => {
            emit_text(report);
            Ok(())
        }
        OutputFormat::Json => {
            emit_json(report);
            Ok(())
        }
        OutputFormat::Both => emit_both(report, json_out),
    }
}

fn emit_text(report: &Report) {
    print_out(&render_text(report));
}

fn emit_json(report: &Report) {
    print_out(&render_json(report));
}

fn emit_both(report: &Report, json_out: Option<&Path>) -> Result<(), CliError> {
    print_out(&render_text(report));
    let json = render_json(report);
    if let Some(path) = json_out {
        fs::write(path, json).map_err(|err| CliError {
            message: format!("failed to write {}: {err}", path.display()),
            exit: ExitCode::from(2),
        })?;
    } else {
        print_err(&json);
    }
    Ok(())
}

/// Writes `message` to stdout.
#[expect(
    clippy::print_stdout,
    reason = "CLI reporter writes the human/JSON report to stdout"
)]
pub fn print_out(message: &str) {
    println!("{message}");
}

/// Writes `message` to stderr.
#[expect(
    clippy::print_stderr,
    reason = "CLI writes usage errors and dual-format JSON fallback to stderr"
)]
pub fn print_err(message: &str) {
    eprintln!("{message}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CliArgs;
    use dry_core::{Config, OutputFormat, ReportSummary};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project() -> std::path::PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let base = std::env::temp_dir().join(format!("dry-rs-runner-{stamp}"));
        assert!(fs::create_dir_all(base.join("src")).is_ok());
        assert!(fs::write(
            base.join("src/lib.rs"),
            "fn one() {\n    let a = 1;\n    let b = a + 1;\n    let c = b + 1;\n}\nfn two() {\n    let a = 1;\n    let b = a + 1;\n    let c = b + 1;\n}\n",
        )
        .is_ok());
        base
    }

    #[test]
    #[expect(
        clippy::cognitive_complexity,
        reason = "runner format matrix is intentionally flat assertions"
    )]
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
        let bad = base.join("missing-dir").join("report.json");
        assert!(emit_report(&report, OutputFormat::Both, Some(&bad)).is_err());
        assert!(exit_for_findings(true, false) == ExitCode::from(1));
        assert!(exit_for_findings(false, false) == ExitCode::SUCCESS);
        print_out("ok");
        print_err("err");
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
