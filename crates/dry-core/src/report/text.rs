//! Human-readable text report renderer.

use std::fmt::Write as _;

use crate::domain::{Finding, Tier};
use crate::report::Report;

/// Renders a multi-section text summary for humans.
#[must_use]
pub fn render_text(report: &Report) -> String {
    let mut out = String::new();
    write_header(&mut out, report);
    write_tier_sections(&mut out, report);
    write_warnings(&mut out, report);
    out
}

fn write_header(out: &mut String, report: &Report) {
    let _ = writeln!(out, "dry-rs report (threshold {:.2})", report.threshold);
    let _ = writeln!(
        out,
        "files={} forms={} findings={} warnings={}\n",
        report.summary.files_scanned,
        report.summary.forms_compared,
        report.summary.total_findings(),
        report.summary.parse_warnings
    );
}

fn write_tier_sections(out: &mut String, report: &Report) {
    for tier in [Tier::AutoRefactor, Tier::ReviewFirst, Tier::Advisory] {
        write_tier_group(out, report, tier);
    }
}

fn write_tier_group(out: &mut String, report: &Report, tier: Tier) {
    let group: Vec<&Finding> = report.findings.iter().filter(|f| f.tier == tier).collect();
    if group.is_empty() {
        return;
    }
    let _ = writeln!(out, "## {}", tier.as_str());
    for finding in group {
        append_finding(out, finding);
    }
    out.push('\n');
}

fn write_warnings(out: &mut String, report: &Report) {
    if report.parse_warnings.is_empty() {
        return;
    }
    out.push_str("## warnings\n");
    for warning in &report.parse_warnings {
        let _ = writeln!(out, "- {warning}");
    }
}

fn append_finding(out: &mut String, finding: &Finding) {
    let _ = writeln!(
        out,
        "- {} score={:.4} {}",
        finding.clone_type.as_str(),
        finding.score,
        finding.tier.as_str()
    );
    for member in &finding.members {
        let _ = writeln!(
            out,
            "    {}:{}-{} {}",
            member.path.display(),
            member.start_line,
            member.end_line,
            member.name
        );
    }
}
