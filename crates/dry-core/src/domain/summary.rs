//! Aggregate counts for a finished report.

use serde::{Deserialize, Serialize};

/// Summary counters embedded in JSON and text reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReportSummary {
    /// Findings routed as `auto_refactor`.
    pub auto_refactor: u32,
    /// Findings routed as `review_first`.
    pub review_first: u32,
    /// Findings routed as `advisory`.
    pub advisory: u32,
    /// Findings labeled Type-1.
    pub type_1: u32,
    /// Findings labeled Type-2.
    pub type_2: u32,
    /// Findings labeled Type-3.
    pub type_3: u32,
    /// Files that returned `Ok` from normalize (includes ignore-file
    /// suppressions and files with no qualifying forms; excludes size skips,
    /// I/O failures, and parse errors).
    pub files_scanned: u32,
    /// Forms that entered comparison.
    pub forms_compared: u32,
    /// Files skipped due to parse or I/O warnings.
    pub parse_warnings: u32,
}

impl ReportSummary {
    /// Total findings across all tiers.
    #[must_use]
    pub const fn total_findings(&self) -> u32 {
        self.auto_refactor
            .saturating_add(self.review_first)
            .saturating_add(self.advisory)
    }
}
