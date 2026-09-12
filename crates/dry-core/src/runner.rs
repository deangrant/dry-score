//! Shared analysis runner and report emission helpers.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::cli::{CliArgs, CliError};
use crate::ports::LanguageNormalizer;
use crate::{OutputFormat, Report, analyze, render_json, render_text};

/// Runs analysis for already-parsed CLI arguments with the given normalizer.
///
/// # Errors
///
/// Returns [`CliError`] when analysis or report emission fails.
pub fn run_analysis(
    args: &CliArgs,
    normalizer: &impl LanguageNormalizer,
) -> Result<ExitCode, CliError> {
    let result =
        analyze(&args.paths, &args.config, normalizer, args.bin_name).map_err(|message| {
            CliError {
                message,
                exit: ExitCode::from(2),
                print_stdout: false,
            }
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

/// Selects the process exit code for a finished analysis run.
#[must_use]
pub fn exit_for_findings(fail_on: bool, empty: bool) -> ExitCode {
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
        OutputFormat::Json => emit_json(report),
        OutputFormat::Both => emit_both(report, json_out),
    }
}

fn emit_text(report: &Report) {
    print_out(&render_text(report));
}

fn emit_json(report: &Report) -> Result<(), CliError> {
    print_out(&json_report(report)?);
    Ok(())
}

fn emit_both(report: &Report, json_out: Option<&Path>) -> Result<(), CliError> {
    print_out(&render_text(report));
    let json = json_report(report)?;
    if let Some(path) = json_out {
        fs::write(path, json).map_err(|err| CliError {
            message: format!("failed to write {}: {err}", path.display()),
            exit: ExitCode::from(2),
            print_stdout: false,
        })?;
    } else {
        print_err(&json);
    }
    Ok(())
}

fn json_report(report: &Report) -> Result<String, CliError> {
    render_json(report).map_err(|message| CliError {
        message: format!("failed to render JSON report: {message}"),
        exit: ExitCode::from(2),
        print_stdout: false,
    })
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

/// Maps a CLI [`Result`] to a process [`ExitCode`], printing messages as needed.
#[must_use]
pub fn exit_from_cli_result(result: Result<ExitCode, CliError>) -> ExitCode {
    match result {
        Ok(code) => code,
        Err(err) => {
            if err.print_stdout {
                print_out(&err.message);
            } else {
                print_err(&err.message);
            }
            err.exit
        }
    }
}

/// Returns true when a form is below configured node or line thresholds.
#[must_use]
pub const fn below_size_thresholds(
    node_count: u32,
    start: u32,
    end: u32,
    min_nodes: u32,
    min_lines: u32,
) -> bool {
    let line_count = end.saturating_sub(start).saturating_add(1);
    node_count < min_nodes || line_count < min_lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ReportSummary;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> std::path::PathBuf {
        // dry-rs:ignore. Per-module test temp-dir helper; shared shape is intentional.
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let base = std::env::temp_dir().join(format!("dry-core-runner-{stamp}"));
        assert!(fs::create_dir_all(&base).is_ok());
        base
    }

    #[test]
    fn emit_formats_cover_variants() {
        let base = temp_dir();
        let report = Report::new(
            "dry-core",
            0.85,
            Vec::new(),
            ReportSummary::default(),
            Vec::new(),
        );
        assert!(emit_report(&report, OutputFormat::Text, None).is_ok());
        assert!(emit_report(&report, OutputFormat::Json, None).is_ok());
        assert!(emit_report(&report, OutputFormat::Both, None).is_ok());
        let out = base.join("report.json");
        assert!(emit_report(&report, OutputFormat::Both, Some(&out)).is_ok());
        let bad = base.join("missing-dir").join("report.json");
        assert!(emit_report(&report, OutputFormat::Both, Some(&bad)).is_err());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn exit_codes_and_print_helpers() {
        assert!(exit_for_findings(true, false) == ExitCode::from(1));
        assert!(exit_for_findings(false, false) == ExitCode::SUCCESS);
        print_out("ok");
        print_err("err");
    }
}
