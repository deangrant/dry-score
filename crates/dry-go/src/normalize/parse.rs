//! Tree-sitter parse helpers for Go sources.

use std::cell::RefCell;

use dry_core::NormalizeError;
use tree_sitter::{Parser, Tree};

thread_local! {
    static PARSER: RefCell<Option<Parser>> = const { RefCell::new(None) };
}

/// Parsed Go CST plus whether the root reported a recoverable error.
#[derive(Debug)]
pub struct ParseResult {
    /// Syntax tree (may contain `ERROR` / missing nodes).
    pub tree: Tree,
    /// True when Tree-sitter marked the root with `has_error`.
    pub has_error: bool,
}

/// Parses Go source into a CST, reusing a thread-local parser.
///
/// Soft-fails on syntax errors: still returns the tree when
/// `root.has_error()` is true so extract can walk the partial CST.
///
/// # Errors
///
/// Returns [`NormalizeError`] when the grammar cannot load or parse returns no
/// tree.
pub fn parse_source(source: &str) -> Result<ParseResult, NormalizeError> {
    PARSER.with(|cell| {
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
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .map_err(|err| NormalizeError::new(format!("language error: {err}")))?;
    Ok(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GoNormalizer;
    use dry_core::LanguageNormalizer;
    use std::path::Path;

    #[test]
    fn soft_error_yields_forms_and_warning() {
        // Recoverable: valid function then junk tokens Tree-sitter marks ERROR.
        let src = r"package p
func ok(n int) int {
  if n < 0 {
    n = 0 - n
  }
  return n + 1
}
func broken( {
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let parsed = parse_source(src).expect("soft parse");
        assert!(parsed.has_error);
        let normalizer = GoNormalizer::new(3, 2);
        let mut next_id = 1;
        #[expect(clippy::expect_used, reason = "test setup")]
        let outcome = normalizer
            .normalize_file(Path::new("partial.go"), src, &mut next_id)
            .expect("normalize");
        assert!(
            outcome.forms.iter().any(|f| f.name == "ok"),
            "forms={:?}",
            outcome.forms
        );
        assert!(
            outcome.warnings.iter().any(|w| w.contains("partial CST")),
            "warnings={:?}",
            outcome.warnings
        );
    }

    #[test]
    fn ignore_file_skips_normalize() {
        let src = "// dry-go:ignore-file\npackage p\nfunc ok() {}\n";
        let normalizer = GoNormalizer::new(1, 1);
        let mut next_id = 1;
        #[expect(clippy::expect_used, reason = "test setup")]
        let outcome = normalizer
            .normalize_file(Path::new("ignored.go"), src, &mut next_id)
            .expect("normalize");
        assert!(outcome.forms.is_empty());
        assert!(outcome.warnings.is_empty());
    }
}
