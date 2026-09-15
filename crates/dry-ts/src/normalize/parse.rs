//! Tree-sitter parse helpers for TypeScript / TSX sources.

use std::cell::RefCell;
use std::path::Path;

use dry_core::NormalizeError;
use tree_sitter::{Parser, Tree};

thread_local! {
    static TS_PARSER: RefCell<Option<Parser>> = const { RefCell::new(None) };
    static TSX_PARSER: RefCell<Option<Parser>> = const { RefCell::new(None) };
}

/// Parsed TypeScript CST plus whether the root reported a recoverable error.
#[derive(Debug)]
pub(super) struct ParseResult {
    /// Syntax tree (may contain `ERROR` / missing nodes).
    pub tree: Tree,
    /// True when Tree-sitter marked the root with `has_error`.
    pub has_error: bool,
}

/// Parses TypeScript or TSX source into a CST, reusing a thread-local parser.
///
/// Soft-fails on syntax errors: still returns the tree when
/// `root.has_error()` is true so extract can walk the partial CST.
///
/// # Errors
///
/// Returns [`NormalizeError`] when the grammar cannot load or parse returns no
/// tree.
pub(super) fn parse_source(path: &Path, source: &str) -> Result<ParseResult, NormalizeError> {
    if is_tsx_path(path) {
        parse_with(&TSX_PARSER, source, GrammarKind::Tsx)
    } else {
        parse_with(&TS_PARSER, source, GrammarKind::Typescript)
    }
}

#[derive(Debug, Clone, Copy)]
enum GrammarKind {
    Typescript,
    Tsx,
}

fn is_tsx_path(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("tsx")
}

fn parse_with(
    slot: &'static std::thread::LocalKey<RefCell<Option<Parser>>>,
    source: &str,
    kind: GrammarKind,
) -> Result<ParseResult, NormalizeError> {
    slot.with(|cell| {
        let mut guard = cell.borrow_mut();
        if guard.is_none() {
            *guard = Some(make_parser(kind)?);
        }
        let parser = guard
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

fn make_parser(kind: GrammarKind) -> Result<Parser, NormalizeError> {
    let mut parser = Parser::new();
    let language = match kind {
        GrammarKind::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        GrammarKind::Tsx => tree_sitter_typescript::LANGUAGE_TSX,
    };
    parser
        .set_language(&language.into())
        .map_err(|err| NormalizeError::new(format!("language error: {err}")))?;
    Ok(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TsNormalizer;
    use dry_core::LanguageNormalizer;
    use std::path::Path;

    #[test]
    fn soft_error_yields_forms_and_warning() {
        let src = r"function ok(n: number): number {
  if (n < 0) {
    n = 0 - n;
  }
  return n + 1;
}
function broken( {
";
        #[expect(clippy::expect_used, reason = "test setup")]
        let parsed = parse_source(Path::new("partial.ts"), src).expect("soft parse");
        assert!(parsed.has_error);
        let normalizer = TsNormalizer::new(3, 2);
        let mut next_id = 1;
        #[expect(clippy::expect_used, reason = "test setup")]
        let outcome = normalizer
            .normalize_file(Path::new("partial.ts"), src, &mut next_id)
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
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go; intentional.
        let src = "// dry-ts:ignore-file\nfunction ok() {}\n";
        let normalizer = TsNormalizer::new(1, 1);
        let mut next_id = 1;
        #[expect(clippy::expect_used, reason = "test setup")]
        let outcome = normalizer
            .normalize_file(Path::new("ignored.ts"), src, &mut next_id)
            .expect("normalize");
        assert!(outcome.forms.is_empty());
        assert!(outcome.warnings.is_empty());
    }

    #[test]
    fn declaration_file_skips_normalize() {
        let src = "export declare function ok(n: number): number;\n";
        let normalizer = TsNormalizer::new(1, 1);
        let mut next_id = 1;
        #[expect(clippy::expect_used, reason = "test setup")]
        let outcome = normalizer
            .normalize_file(Path::new("types.d.ts"), src, &mut next_id)
            .expect("normalize");
        assert!(outcome.forms.is_empty());
    }

    #[test]
    fn tsx_path_uses_jsx_grammar() {
        let src = "export function App() { return <div>hi</div>; }\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let parsed = parse_source(Path::new("App.tsx"), src).expect("tsx parse");
        assert!(!parsed.has_error);
    }
}
