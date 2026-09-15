//! Tree-sitter parse helpers for Python sources.

use std::cell::RefCell;

use dry_core::NormalizeError;
use tree_sitter::{Parser, Tree};

thread_local! {
    static PARSER: RefCell<Option<Parser>> = const { RefCell::new(None) };
}

/// Parsed Python CST plus whether the root reported a recoverable error.
#[derive(Debug)]
pub(super) struct ParseResult {
    /// Syntax tree (may contain `ERROR` / missing nodes).
    pub tree: Tree,
    /// True when Tree-sitter marked the root with `has_error`.
    pub has_error: bool,
}

/// Parses Python source into a CST, reusing a thread-local parser.
///
/// Callers must treat `has_error` as a hard failure and discard forms; this
/// helper still returns the tree so diagnostics can inspect the root flag.
///
/// # Errors
///
/// Returns [`NormalizeError`] when the grammar cannot load or parse returns no
/// tree.
pub(super) fn parse_source(source: &str) -> Result<ParseResult, NormalizeError> {
    PARSER.with(|cell| {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(make_parser()?);
        }
        let parser = slot
            .as_mut()
            .ok_or_else(|| NormalizeError::new("parser slot missing after init"))?;
        parser.reset();
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| NormalizeError::new("parse returned no tree"))?;
        let has_error = tree.root_node().has_error();
        Ok(ParseResult { tree, has_error })
    })
}

fn make_parser() -> Result<Parser, NormalizeError> {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .map_err(|err| NormalizeError::new(format!("language error: {err}")))?;
    Ok(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PyNormalizer;
    use dry_core::LanguageNormalizer;
    use std::path::Path;

    #[test]
    fn parse_error_discards_forms() {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
        let src = r"def ok(n):
    if n < 0:
        n = 0 - n
    return n + 1

def broken(
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let parsed = parse_source(src).expect("parse tree");
        assert!(parsed.has_error);
        let normalizer = PyNormalizer::new(3, 2);
        let mut next_id = 1;
        let outcome = normalizer.normalize_file(Path::new("partial.py"), src, &mut next_id);
        assert!(outcome.is_err());
    }

    #[test]
    fn ignore_file_skips_normalize() {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
        let src = "# dry-py:ignore-file\ndef ok():\n    pass\n";
        let normalizer = PyNormalizer::new(1, 1);
        let mut next_id = 1;
        #[expect(clippy::expect_used, reason = "test setup")]
        let outcome = normalizer
            .normalize_file(Path::new("ignored.py"), src, &mut next_id)
            .expect("normalize");
        assert!(outcome.forms.is_empty());
        assert!(outcome.warnings.is_empty());
    }

    #[test]
    fn stub_file_skips_normalize() {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
        let src = "def ok(n: int) -> int: ...\n";
        let normalizer = PyNormalizer::new(1, 1);
        let mut next_id = 1;
        #[expect(clippy::expect_used, reason = "test setup")]
        let outcome = normalizer
            .normalize_file(Path::new("types.pyi"), src, &mut next_id)
            .expect("normalize");
        assert!(outcome.forms.is_empty());
    }
}
