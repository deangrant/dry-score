//! Suppression markers in source text.
//!
//! Markers must appear as a full-line `//` comment directive (optional leading
//! whitespace). Trailing comments and string/URL substrings do not count.

/// Returns true when the file opts out of analysis entirely.
#[must_use]
pub fn file_is_ignored(source: &str) -> bool {
    source
        .lines()
        .take(40)
        .any(|line| directive_matches(line, "dry-rs:ignore-file"))
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
        if directive_matches(line, "dry-rs:ignore") {
            return true;
        }
    }
    false
}

fn directive_matches(line: &str, marker: &str) -> bool {
    let Some(body) = full_line_comment_body(line) else {
        return false;
    };
    body.starts_with(marker) && !token_continues(&body[marker.len()..])
}

fn full_line_comment_body(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("//")?;
    Some(rest.trim_start())
}

fn token_continues(rest: &str) -> bool {
    rest.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_and_span_ignore_markers() {
        assert!(file_is_ignored("// dry-rs:ignore-file\nfn a() {}\n"));
        assert!(file_is_ignored("  // dry-rs:ignore-file\nfn a() {}\n"));
        assert!(!file_is_ignored("fn a() {}\n"));
        assert!(span_is_ignored("a\n// dry-rs:ignore\nb\n", 2, 2));
        assert!(span_is_ignored("a\n// dry-rs:ignore. reason\nb\n", 2, 2));
        assert!(!span_is_ignored("a\nb\nc\n", 1, 2));
        assert!(!span_is_ignored("a\nb\n// dry-rs:ignore\n", 1, 2));
    }

    #[test]
    fn rejects_substring_false_positives() {
        assert!(!span_is_ignored("let s = \"dry-rs:ignore\";\n", 1, 1));
        assert!(!span_is_ignored(
            "let u = \"http://dry-rs:ignore\";\n",
            1,
            1
        ));
        assert!(!file_is_ignored("println!(\"dry-rs:ignore-file\");\n"));
        assert!(!span_is_ignored("  let dry_rs_ignore = 1;\n", 1, 1));
        assert!(!span_is_ignored("// dry-rs:ignore-file\n", 1, 1));
        assert!(!span_is_ignored("code; // dry-rs:ignore\n", 1, 1));
    }
}
