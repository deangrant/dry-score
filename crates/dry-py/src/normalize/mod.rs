//! Python Tree-sitter normalizer for structural forms.
//!
//! Normalization rules:
//! - Comments are skipped in the CST walk.
//! - Identifiers become positional placeholders (`id0`, `id1`, …).
//! - Raw spellings stay in `ident_trace` for Type-1 versus Type-2.
//! - Literals become kind tags (`lit_int`, `lit_str`, …).
//! - Control flow and operators keep structural labels.
//! - Fingerprints use shared FNV-1a via `dry_core::fingerprint_tree`.

mod emit;
mod extract;
mod parse;
mod suppress;

use std::path::Path;

use dry_core::{FormKind, LanguageNormalizer, NormalizeError, NormalizeOutcome, file_is_ignored};

use extract::extract_forms;
use parse::parse_source;
use suppress::MARKER;

/// Python source normalizer backed by Tree-sitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PyNormalizer {
    /// Minimum structural nodes required to emit a form.
    pub min_nodes: u32,
    /// Minimum source lines required to emit a form.
    pub min_lines: u32,
}

impl PyNormalizer {
    /// Builds a normalizer with size thresholds.
    #[must_use]
    pub const fn new(min_nodes: u32, min_lines: u32) -> Self {
        Self {
            min_nodes,
            min_lines,
        }
    }
}

impl LanguageNormalizer for PyNormalizer {
    fn normalize_file(
        &self,
        path: &Path,
        source: &str,
        next_id: &mut u64,
    ) -> Result<NormalizeOutcome, NormalizeError> {
        // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
        if file_is_ignored(source, MARKER) || is_stub_file(path) {
            return Ok(NormalizeOutcome {
                forms: Vec::new(),
                warnings: Vec::new(),
            });
        }
        let parsed = parse_source(source)?;
        if parsed.has_error {
            return Err(NormalizeError::new(
                "parse error in Python source; forms discarded",
            ));
        }
        let forms = extract_forms(
            parsed.tree.root_node(),
            path,
            source.as_bytes(),
            source,
            self.min_nodes,
            self.min_lines,
            next_id,
        );
        Ok(NormalizeOutcome {
            forms,
            warnings: Vec::new(),
        })
    }
}

/// Classifies production versus test from Python path conventions.
#[must_use]
pub fn kind_from_path(path: &Path) -> FormKind {
    if is_test_filename(path) || has_test_path_component(path) {
        FormKind::Test
    } else {
        FormKind::Production
    }
}

fn is_test_filename(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    name.starts_with("test_") || name.ends_with("_test.py")
}

fn has_test_path_component(path: &Path) -> bool {
    path.components().any(|component| {
        let comp = component.as_os_str();
        comp == "tests" || comp == "test"
    })
}

/// Stub files end in `.pyi` and are skipped (no executable bodies).
#[must_use]
pub fn is_stub_file(path: &Path) -> bool {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    path.extension().and_then(|s| s.to_str()) == Some("pyi")
}
