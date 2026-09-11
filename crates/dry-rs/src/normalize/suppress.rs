//! Suppression markers in Rust source (`dry-rs:ignore`).

use dry_core::{file_is_ignored as core_file_ignored, span_is_ignored as core_span_ignored};

const MARKER: &str = "dry-rs:ignore";

/// Returns true when the file opts out of analysis entirely.
#[must_use]
pub fn file_is_ignored(source: &str) -> bool {
    core_file_ignored(source, MARKER)
}

/// Returns true when any line in `[start_line, end_line]` opts out.
#[must_use]
pub fn span_is_ignored(source: &str, start_line: u32, end_line: u32) -> bool {
    core_span_ignored(source, start_line, end_line, MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_and_span_ignore_markers() {
        assert!(file_is_ignored("// dry-rs:ignore-file\nfn a() {}\n"));
        assert!(span_is_ignored("a\n// dry-rs:ignore\nb\n", 2, 2));
    }
}
