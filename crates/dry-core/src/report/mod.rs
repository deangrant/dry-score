//! Report envelope and renderers.

mod json;
mod text;

use serde::{Deserialize, Serialize};

use crate::domain::{Finding, ReportSummary};

#[doc(inline)]
pub use json::render_json;
#[doc(inline)]
pub use text::render_text;

/// Versioned analysis report shared by JSON and text renderers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// Wire format version.
    pub version: String,
    /// Threshold used for this run.
    pub threshold: f64,
    /// Findings sorted most exact to least exact.
    pub findings: Vec<Finding>,
    /// Aggregate counters.
    pub summary: ReportSummary,
    /// Parse or I/O warnings encountered during the run.
    pub parse_warnings: Vec<String>,
}

impl Report {
    /// Builds a v0.1 report envelope.
    #[must_use]
    pub fn new(
        threshold: f64,
        findings: Vec<Finding>,
        summary: ReportSummary,
        parse_warnings: Vec<String>,
    ) -> Self {
        Self {
            version: "0.1".to_owned(),
            threshold,
            findings,
            summary,
            parse_warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CloneType, Finding, FormKind, FormMember, FormSpan, ReportSummary, Tier};
    use std::path::PathBuf;

    fn sample_report() -> Report {
        let finding = Finding {
            clone_type: CloneType::Type2,
            tier: Tier::ReviewFirst,
            score: 0.9,
            members: vec![FormMember::new(
                PathBuf::from("a.rs"),
                FormSpan::new(1, 5),
                "foo".to_owned(),
            )],
        };
        let summary = ReportSummary {
            review_first: 1,
            type_2: 1,
            files_scanned: 2,
            forms_compared: 4,
            parse_warnings: 1,
            ..ReportSummary::default()
        };
        Report::new(0.85, vec![finding], summary, vec!["warn".to_owned()])
    }

    #[test]
    fn render_text_includes_tiers_and_warnings() {
        let text = render_text(&sample_report());
        assert!(text.contains("dry-rs report"));
        assert!(text.contains("## review_first"));
        assert!(text.contains("## warnings"));
        assert!(text.contains("type_2"));
        assert_eq!(FormSpan::new(2, 4).line_count(), 3);
        assert_eq!(sample_report().summary.total_findings(), 1);
        let _ = FormKind::Production;
    }

    #[test]
    fn render_json_round_trips_version() {
        let json = render_json(&sample_report());
        assert!(json.contains("\"version\": \"0.1\""));
    }

    #[test]
    fn render_text_skips_empty_tiers() {
        let report = Report::new(0.5, Vec::new(), ReportSummary::default(), Vec::new());
        let text = render_text(&report);
        assert!(!text.contains("## auto_refactor"));
        assert!(!text.contains("## warnings"));
    }
}
