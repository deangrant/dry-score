//! JSON report renderer.

use crate::report::Report;

/// Serializes a report to pretty JSON.
///
/// # Errors
///
/// Returns a string when serialization fails. Callers should fail the run
/// rather than inventing an alternate error schema.
pub fn render_json(report: &Report) -> Result<String, String> {
    serde_json::to_string_pretty(report).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ReportSummary;
    use crate::report::Report;

    #[test]
    fn render_json_ok_for_empty_report() {
        let report = Report::new(
            "dry-core",
            0.85,
            Vec::new(),
            ReportSummary::default(),
            Vec::new(),
        );
        let json = render_json(&report);
        assert!(json.is_ok());
        #[expect(clippy::expect_used, reason = "test asserts serialize ok")]
        let json = json.expect("ok");
        assert!(json.contains("\"version\""));
        assert!(json.contains("\"tool\""));
    }
}
