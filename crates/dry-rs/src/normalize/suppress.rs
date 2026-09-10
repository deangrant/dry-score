//! Suppression markers in source text.

/// Returns true when the file opts out of analysis entirely.
#[must_use]
pub fn file_is_ignored(source: &str) -> bool {
    source.lines().take(40).any(|line| line.contains("dry-rs:ignore-file"))
}

/// Returns true when any line in `[start_line, end_line]` opts out.
#[must_use]
pub fn span_is_ignored(source: &str, start_line: u32, end_line: u32) -> bool {
    for (idx, line) in source.lines().enumerate() {
        let line_no = u32::try_from(idx.saturating_add(1)).unwrap_or(u32::MAX);
        if line_no < start_line {
            continue;
        }
        if line_no > end_line {
            break;
        }
        if line.contains("dry-rs:ignore") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_and_span_ignore_markers() {
        assert!(file_is_ignored("// dry-rs:ignore-file\nfn a() {}\n"));
        assert!(!file_is_ignored("fn a() {}\n"));
        assert!(span_is_ignored("a\n// dry-rs:ignore\nb\n", 2, 2));
        assert!(!span_is_ignored("a\nb\nc\n", 1, 2));
        assert!(!span_is_ignored("a\nb\n// dry-rs:ignore\n", 1, 2));
    }
}
