//! Source span for a normalized form.

use serde::{Deserialize, Serialize};

/// One-based line span inside a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FormSpan {
    /// Inclusive start line (1-based).
    pub start_line: u32,
    /// Inclusive end line (1-based).
    pub end_line: u32,
}

impl FormSpan {
    /// Builds a span from start and end lines.
    #[must_use]
    pub const fn new(start_line: u32, end_line: u32) -> Self {
        Self {
            start_line,
            end_line,
        }
    }

    /// Number of lines covered by this span.
    #[must_use]
    pub const fn line_count(self) -> u32 {
        self.end_line.saturating_sub(self.start_line).saturating_add(1)
    }
}
