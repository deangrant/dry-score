//! Go Tree-sitter normalizer for structural forms.
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

const MARKER: &str = "dry-go:ignore";

/// Go source normalizer backed by Tree-sitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoNormalizer {
    /// Minimum structural nodes required to emit a form.
    pub min_nodes: u32,
    /// Minimum source lines required to emit a form.
    pub min_lines: u32,
}

impl GoNormalizer {
    /// Builds a normalizer with size thresholds.
    #[must_use]
    pub const fn new(min_nodes: u32, min_lines: u32) -> Self {
        Self {
            min_nodes,
            min_lines,
        }
    }
}

impl LanguageNormalizer for GoNormalizer {
    fn normalize_file(
        &self,
        path: &Path,
        source: &str,
        next_id: &mut u64,
    ) -> Result<NormalizeOutcome, NormalizeError> {
        if file_is_ignored(source, MARKER) {
            return Ok(NormalizeOutcome {
                forms: Vec::new(),
                warnings: Vec::new(),
            });
        }
        let parsed = parse_source(source)?;
        let forms = extract_forms(
            parsed.tree.root_node(),
            path,
            source.as_bytes(),
            source,
            self.min_nodes,
            self.min_lines,
            next_id,
        );
        let mut warnings = Vec::new();
        if parsed.has_error {
            warnings.push("parse error in Go source; extracted from partial CST".to_owned());
        }
        Ok(NormalizeOutcome { forms, warnings })
    }
}

/// Classifies production versus test from the Go path convention.
#[must_use]
pub fn kind_from_path(path: &Path) -> FormKind {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    if name.ends_with("_test.go") {
        FormKind::Test
    } else {
        FormKind::Production
    }
}
