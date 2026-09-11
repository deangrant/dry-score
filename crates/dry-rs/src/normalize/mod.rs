//! Rust `syn`-based normalizer for structural forms.
//!
//! Normalization rules (reproducibility):
//! - Comments and whitespace are discarded by the parser.
//! - Identifiers become positional placeholders (`id0`, `id1`, …) in visit order,
//!   so renamed twins share fingerprints.
//! - Raw identifier spellings are kept in `ident_trace` (occurrence sequence,
//!   not a unique set) for Type-1 vs Type-2.
//! - Literals become kind tags (`lit_int`, `lit_str`, …).
//! - Control-flow and operator nodes keep their structural labels.
//! - Patterns emit structural nodes (or/range/slice/ref/…), not a catch-all.
//! - Macros fingerprint as name + delimiter + token-tree structure (not expansion).
//! - Trait default method bodies are extracted as named forms.
//! - Each subtree hashes to a `u64` via fixed FNV-1a; all subtree hashes form
//!   the fingerprint set.

mod emit;
mod extract;
mod placeholders;
mod suppress;

use std::path::Path;

use dry_core::{
    FormKind, FormSpan, LanguageNormalizer, NormalizeError, NormalizeOutcome, NormalizedForm,
};

use extract::extract_forms;
use suppress::file_is_ignored;

/// Rust source normalizer backed by `syn`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RustNormalizer {
    /// Minimum structural nodes required to emit a form.
    pub min_nodes: u32,
    /// Minimum source lines required to emit a form.
    pub min_lines: u32,
}

impl RustNormalizer {
    /// Builds a normalizer with size thresholds.
    #[must_use]
    pub const fn new(min_nodes: u32, min_lines: u32) -> Self {
        Self {
            min_nodes,
            min_lines,
        }
    }
}

impl LanguageNormalizer for RustNormalizer {
    fn normalize_file(
        &self,
        path: &Path,
        source: &str,
        next_id: &mut u64,
    ) -> Result<NormalizeOutcome, NormalizeError> {
        if file_is_ignored(source) {
            return Ok(NormalizeOutcome { forms: Vec::new() });
        }
        let file = syn::parse_file(source)
            .map_err(|err| NormalizeError::new(format!("parse error: {err}")))?;
        let forms = extract_forms(&file, path, source, self.min_nodes, self.min_lines, next_id);
        Ok(NormalizeOutcome { forms })
    }
}

/// Convenience helper for production vs test kind.
#[must_use]
pub const fn kind_from_test(is_test: bool) -> FormKind {
    if is_test {
        FormKind::Test
    } else {
        FormKind::Production
    }
}

/// Parts required to construct a [`NormalizedForm`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormParts {
    /// Stable identity.
    pub id: u64,
    /// Display name.
    pub name: String,
    /// Source path.
    pub path: std::path::PathBuf,
    /// Start line.
    pub start_line: u32,
    /// End line.
    pub end_line: u32,
    /// Production or test.
    pub kind: FormKind,
    /// Structural node count.
    pub node_count: u32,
    /// Fingerprint set.
    pub fingerprints: std::collections::BTreeSet<u64>,
    /// Raw identifier occurrence sequence (repeats included).
    pub ident_trace: Vec<String>,
}

/// Builds a [`NormalizedForm`] from prepared parts.
#[must_use]
pub fn build_form(parts: FormParts) -> NormalizedForm {
    NormalizedForm {
        id: parts.id,
        name: parts.name,
        path: parts.path,
        span: FormSpan::new(parts.start_line, parts.end_line),
        kind: parts.kind,
        node_count: parts.node_count,
        fingerprints: parts.fingerprints,
        ident_trace: parts.ident_trace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dry_core::LanguageNormalizer;
    use std::path::Path;

    #[test]
    fn normalize_file_and_helpers() {
        let normalizer = RustNormalizer::new(5, 3);
        let mut next_id = 1;
        let source = "fn big() {\n    let a = 1;\n    let b = a + 1;\n    let c = b + 2;\n}\n";
        let outcome = normalizer.normalize_file(Path::new("a.rs"), source, &mut next_id);
        assert!(outcome.is_ok());
        let ignored = normalizer.normalize_file(
            Path::new("a.rs"),
            "// dry-rs:ignore-file\nfn x() {}\n",
            &mut next_id,
        );
        assert!(ignored.is_ok());
        #[expect(clippy::expect_used, reason = "test")]
        let ignored = ignored.expect("ok");
        assert!(ignored.forms.is_empty());
        let bad = normalizer.normalize_file(Path::new("a.rs"), "fn (", &mut next_id);
        assert!(bad.is_err());
        assert_eq!(kind_from_test(true), FormKind::Test);
        assert_eq!(kind_from_test(false), FormKind::Production);
        let form = build_form(FormParts {
            id: 1,
            name: "n".to_owned(),
            path: Path::new("a.rs").to_path_buf(),
            start_line: 1,
            end_line: 2,
            kind: FormKind::Production,
            node_count: 3,
            fingerprints: std::collections::BTreeSet::new(),
            ident_trace: Vec::new(),
        });
        assert_eq!(form.name, "n");
    }
}
