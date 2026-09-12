//! Suppression markers for Go (`dry-go:ignore`).

use dry_core::span_is_ignored as core_span_ignored;

const MARKER: &str = "dry-go:ignore";

/// Returns true when any line in `[start_line, end_line]` opts out.
#[must_use]
pub fn span_is_ignored(source: &str, start_line: u32, end_line: u32) -> bool {
    core_span_ignored(source, start_line, end_line, MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dry_core::file_is_ignored;

    #[test]
    fn go_ignore_markers() {
        assert!(file_is_ignored(
            "// dry-go:ignore-file\nfunc a() {}\n",
            MARKER
        ));
        assert!(span_is_ignored("a\n// dry-go:ignore\nb\n", 2, 2));
    }
}
