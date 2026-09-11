//! Tree-sitter parse helpers for Go sources.

use dry_core::NormalizeError;
use tree_sitter::{Parser, Tree};

/// Parses Go source into a CST; fails when the root reports an error.
///
/// # Errors
///
/// Returns [`NormalizeError`] when the grammar cannot load, parse fails, or the
/// root node has an error.
pub fn parse_source(source: &str) -> Result<Tree, NormalizeError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .map_err(|err| NormalizeError::new(format!("language error: {err}")))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| NormalizeError::new("parse returned no tree"))?;
    if tree.root_node().has_error() {
        return Err(NormalizeError::new("parse error in Go source"));
    }
    Ok(tree)
}
