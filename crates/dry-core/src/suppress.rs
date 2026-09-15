//! Suppression markers shared by language adapters.
//!
//! Markers must appear as a full-line comment directive (optional leading
//! whitespace): `//`, `///`, `//!`, `#`, or a whole-line `/* … */` / `/** … */`.
//! Trailing comments and string/URL substrings do not count.

/// Number of leading source lines scanned for `{marker}-file` directives.
pub const FILE_IGNORE_SCAN_LINES: usize = 40;

/// Returns true when the file opts out via `{marker}-file`.
///
/// Only the first [`FILE_IGNORE_SCAN_LINES`] lines are searched so long
/// license/header blocks do not hide a late file-level directive by accident;
/// place `*-ignore-file` near the top of the file.
#[must_use]
pub fn file_is_ignored(source: &str, marker: &str) -> bool {
    let file_marker = format!("{marker}-file");
    source
        .lines()
        .take(FILE_IGNORE_SCAN_LINES)
        .any(|line| directive_matches(line, &file_marker))
}

/// Returns true when any line in `[start_line, end_line]` opts out via `marker`.
#[must_use]
pub fn span_is_ignored(source: &str, start_line: u32, end_line: u32, marker: &str) -> bool {
    for (idx, line) in source.lines().enumerate() {
        let line_no = u32::try_from(idx.saturating_add(1)).unwrap_or(u32::MAX);
        if line_no < start_line {
            continue;
        }
        if line_no > end_line {
            break;
        }
        if directive_matches(line, marker) {
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
    if let Some(rest) = trimmed.strip_prefix("//") {
        // Allow `///` and `//!` by consuming one extra `/` or `!`.
        let rest = rest.strip_prefix('/').or_else(|| rest.strip_prefix('!')).unwrap_or(rest);
        return Some(rest.trim_start());
    }
    if let Some(rest) = trimmed.strip_prefix('#') {
        return Some(rest.trim_start());
    }
    block_comment_body(trimmed)
}

fn block_comment_body(trimmed: &str) -> Option<&str> {
    let closed = trimmed.trim_end();
    let inner = closed.strip_prefix("/*")?.strip_suffix("*/")?;
    let inner = inner.strip_prefix('*').unwrap_or(inner);
    Some(inner.trim())
}

fn token_continues(rest: &str) -> bool {
    rest.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKER: &str = "dry-rs:ignore";

    #[test]
    fn file_and_span_ignore_markers() {
        assert!(file_is_ignored(
            "// dry-rs:ignore-file\nfn a() {}\n",
            MARKER
        ));
        assert!(file_is_ignored(
            "  // dry-rs:ignore-file\nfn a() {}\n",
            MARKER
        ));
        assert!(!file_is_ignored("fn a() {}\n", MARKER));
        assert!(span_is_ignored("a\n// dry-rs:ignore\nb\n", 2, 2, MARKER));
        assert!(span_is_ignored(
            "a\n// dry-rs:ignore. reason\nb\n",
            2,
            2,
            MARKER
        ));
        assert!(!span_is_ignored("a\nb\nc\n", 1, 2, MARKER));
        assert!(!span_is_ignored("a\nb\n// dry-rs:ignore\n", 1, 2, MARKER));
    }

    #[test]
    fn accepts_doc_and_block_comment_directives() {
        assert!(file_is_ignored(
            "/// dry-rs:ignore-file\nfn a() {}\n",
            MARKER
        ));
        assert!(file_is_ignored(
            "//! dry-rs:ignore-file\nfn a() {}\n",
            MARKER
        ));
        assert!(span_is_ignored("a\n/* dry-rs:ignore */\nb\n", 2, 2, MARKER));
        assert!(span_is_ignored(
            "a\n/** dry-rs:ignore. reason */\nb\n",
            2,
            2,
            MARKER
        ));
    }

    #[test]
    fn accepts_hash_comment_directives() {
        // dry-rs:ignore. Hash-comment suppress corpus; parallel with // /* tests intentional.
        assert!(file_is_ignored(
            "# dry-rs:ignore-file\ndef a():\n    pass\n",
            MARKER
        ));
        assert!(file_is_ignored(
            "  # dry-rs:ignore-file\ndef a():\n    pass\n",
            MARKER
        ));
        assert!(span_is_ignored("a\n# dry-rs:ignore\nb\n", 2, 2, MARKER));
        assert!(span_is_ignored(
            "a\n# dry-rs:ignore. reason\nb\n",
            2,
            2,
            MARKER
        ));
    }

    #[test]
    fn rejects_substring_false_positives() {
        assert!(!span_is_ignored(
            "let s = \"dry-rs:ignore\";\n",
            1,
            1,
            MARKER
        ));
        assert!(!span_is_ignored(
            "let u = \"http://dry-rs:ignore\";\n",
            1,
            1,
            MARKER
        ));
        assert!(!file_is_ignored(
            "println!(\"dry-rs:ignore-file\");\n",
            MARKER
        ));
        assert!(!span_is_ignored("  let dry_rs_ignore = 1;\n", 1, 1, MARKER));
        assert!(!span_is_ignored("// dry-rs:ignore-file\n", 1, 1, MARKER));
        assert!(!span_is_ignored("code; // dry-rs:ignore\n", 1, 1, MARKER));
        assert!(!span_is_ignored(
            "code; /* dry-rs:ignore */\n",
            1,
            1,
            MARKER
        ));
    }

    #[test]
    fn rejects_hash_comment_false_positives() {
        assert!(!span_is_ignored("code  # dry-rs:ignore\n", 1, 1, MARKER));
        assert!(!span_is_ignored("s = \"# dry-rs:ignore\"\n", 1, 1, MARKER));
    }

    #[test]
    fn file_ignore_marker_beyond_scan_window_is_ignored() {
        use std::fmt::Write as _;
        let mut src = String::new();
        for i in 1..=FILE_IGNORE_SCAN_LINES {
            let _ = writeln!(src, "// preamble {i}");
        }
        src.push_str("// dry-rs:ignore-file\nfn a() {}\n");
        assert!(!file_is_ignored(&src, MARKER));
    }

    #[test]
    fn file_ignore_marker_on_last_scanned_line_is_honored() {
        use std::fmt::Write as _;
        let mut src = String::new();
        for i in 1..FILE_IGNORE_SCAN_LINES {
            let _ = writeln!(src, "// preamble {i}");
        }
        src.push_str("// dry-rs:ignore-file\nfn a() {}\n");
        assert!(file_is_ignored(&src, MARKER));
    }
}
