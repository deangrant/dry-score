//! JSON report renderer.

use crate::report::Report;

/// Serializes a report to pretty JSON.
#[must_use]
pub fn render_json(report: &Report) -> String {
    fallback_json(serialize_report(report))
}

fn serialize_report(report: &Report) -> Result<String, String> {
    serde_json::to_string_pretty(report).map_err(|err| err.to_string())
}

fn fallback_json(result: Result<String, String>) -> String {
    result.unwrap_or_else(|message| format!(r#"{{"error":"{message}"}}"#))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_json_covers_error_arm() {
        let json = fallback_json(Err("boom".to_owned()));
        assert!(json.contains("boom"));
    }
}
