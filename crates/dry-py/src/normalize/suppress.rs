//! Suppression markers for Python (`dry-py:ignore`).

use dry_core::span_is_ignored as core_span_ignored;

/// Full-line ignore marker shared by file and span suppress checks.
pub(super) const MARKER: &str = "dry-py:ignore";

/// Returns true when any line in `[start_line, end_line]` opts out.
#[must_use]
pub(super) fn span_is_ignored(source: &str, start_line: u32, end_line: u32) -> bool {
    core_span_ignored(source, start_line, end_line, MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dry_core::file_is_ignored;

    #[test]
    fn py_ignore_markers() {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
        assert!(file_is_ignored(
            "# dry-py:ignore-file\ndef a():\n    pass\n",
            MARKER
        ));
        assert!(span_is_ignored("a\n# dry-py:ignore\nb\n", 2, 2));
    }
}
