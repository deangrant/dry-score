//! End-to-end analysis orchestration.

use std::fs;
use std::path::{Path, PathBuf};

use crate::compare::compare;
use crate::config::Config;
use crate::domain::{CloneType, Finding, NormalizedForm, ReportSummary, Tier};
use crate::ports::LanguageNormalizer;
use crate::report::Report;
use crate::walk::{WalkOptions, collect_source_files};

/// Result of a full analysis run.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisResult {
    /// Finished report envelope.
    pub report: Report,
}

/// Runs discovery, normalization, comparison, and summary building.
///
/// # Errors
///
/// Returns a string when filesystem discovery fails hard.
pub fn analyze(
    roots: &[PathBuf],
    config: &Config,
    normalizer: &impl LanguageNormalizer,
    tool: &str,
) -> Result<AnalysisResult, String> {
    let options = WalkOptions::new(config.walk.extensions.clone(), config.walk.exclude.clone());
    let files = collect_source_files(roots, &options).map_err(|err| err.to_string())?;
    let (forms, warnings, files_scanned) =
        normalize_sources(&files, roots, normalizer, config.walk.max_file_bytes);
    let forms_compared = u32::try_from(forms.len()).unwrap_or(u32::MAX);
    let findings = compare(&forms, config.gate.threshold);
    let summary = build_summary(
        &findings,
        files_scanned,
        forms_compared,
        u32::try_from(warnings.len()).unwrap_or(u32::MAX),
    );
    let report = Report::new(tool, config.gate.threshold, findings, summary, warnings);
    Ok(AnalysisResult { report })
}

/// Prefer root-relative paths for report stability; keep the original when no
/// root is a prefix.
fn relativize_one(path: &Path, roots: &[PathBuf]) -> PathBuf {
    roots
        .iter()
        .filter_map(|root| {
            path.strip_prefix(root)
                .ok()
                .map(|rel| (root.as_os_str().len(), rel.to_path_buf()))
        })
        .max_by_key(|(len, _)| *len)
        .map_or_else(|| path.to_path_buf(), |(_, rel)| rel)
}

fn normalize_sources(
    files: &[PathBuf],
    roots: &[PathBuf],
    normalizer: &impl LanguageNormalizer,
    max_file_bytes: u64,
) -> (Vec<NormalizedForm>, Vec<String>, u32) {
    let mut forms = Vec::new();
    let mut warnings = Vec::new();
    let mut next_id = 1_u64;
    let mut files_scanned = 0_u32;
    let mut state = NormalizeState {
        next_id: &mut next_id,
        forms: &mut forms,
        warnings: &mut warnings,
        files_scanned: &mut files_scanned,
    };
    for path in files {
        let report_path = relativize_one(path, roots);
        normalize_one(path, &report_path, normalizer, max_file_bytes, &mut state);
    }
    (forms, warnings, files_scanned)
}

struct NormalizeState<'a> {
    next_id: &'a mut u64,
    forms: &'a mut Vec<NormalizedForm>,
    warnings: &'a mut Vec<String>,
    files_scanned: &'a mut u32,
}

fn normalize_one(
    path: &Path,
    report_path: &Path,
    normalizer: &impl LanguageNormalizer,
    max_file_bytes: u64,
    state: &mut NormalizeState<'_>,
) {
    if let Err(warning) = check_file_size(path, report_path, max_file_bytes) {
        state.warnings.push(warning);
        return;
    }
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            state.warnings.push(format!("{}: {err}", report_path.display()));
            return;
        }
    };
    apply_normalize_outcome(report_path, normalizer, &source, state);
}

fn apply_normalize_outcome(
    report_path: &Path,
    normalizer: &impl LanguageNormalizer,
    source: &str,
    state: &mut NormalizeState<'_>,
) {
    match normalizer.normalize_file(report_path, source, state.next_id) {
        Ok(outcome) => record_normalize_ok(report_path, outcome, state),
        Err(err) => state.warnings.push(format!("{}: {err}", report_path.display())),
    }
}

fn record_normalize_ok(
    report_path: &Path,
    outcome: crate::ports::NormalizeOutcome,
    state: &mut NormalizeState<'_>,
) {
    *state.files_scanned = state.files_scanned.saturating_add(1);
    for warning in outcome.warnings {
        state.warnings.push(format!("{}: {warning}", report_path.display()));
    }
    state.forms.extend(outcome.forms);
}

fn check_file_size(path: &Path, report_path: &Path, max_file_bytes: u64) -> Result<(), String> {
    let meta = fs::metadata(path).map_err(|err| format!("{}: {err}", report_path.display()))?;
    let len = meta.len();
    if len > max_file_bytes {
        return Err(format!(
            "{}: file exceeds walk.max_file_bytes ({len} > {max_file_bytes})",
            report_path.display()
        ));
    }
    Ok(())
}

fn build_summary(
    findings: &[Finding],
    files_scanned: u32,
    forms_compared: u32,
    parse_warnings: u32,
) -> ReportSummary {
    let mut summary = ReportSummary {
        files_scanned,
        forms_compared,
        parse_warnings,
        ..ReportSummary::default()
    };
    for finding in findings {
        bump_tier(&mut summary, finding.tier);
        bump_clone_type(&mut summary, finding.clone_type);
    }
    summary
}

const fn bump_tier(summary: &mut ReportSummary, tier: Tier) {
    // dry-rs:ignore. Distinct enum arms; shared saturating_add shape is intentional.
    match tier {
        Tier::AutoRefactor => summary.auto_refactor = summary.auto_refactor.saturating_add(1),
        Tier::ReviewFirst => summary.review_first = summary.review_first.saturating_add(1),
        Tier::Advisory => summary.advisory = summary.advisory.saturating_add(1),
    }
}

const fn bump_clone_type(summary: &mut ReportSummary, clone_type: CloneType) {
    // dry-rs:ignore. Distinct enum arms; shared saturating_add shape is intentional.
    match clone_type {
        CloneType::Type1 => summary.type_1 = summary.type_1.saturating_add(1),
        CloneType::Type2 => summary.type_2 = summary.type_2.saturating_add(1),
        CloneType::Type3 => summary.type_3 = summary.type_3.saturating_add(1),
    }
}

#[cfg(test)]
mod tests;
