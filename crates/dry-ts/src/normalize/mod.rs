//! TypeScript Tree-sitter normalizer for structural forms.
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

const MARKER: &str = "dry-ts:ignore";

/// TypeScript source normalizer backed by Tree-sitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TsNormalizer {
    /// Minimum structural nodes required to emit a form.
    pub min_nodes: u32,
    /// Minimum source lines required to emit a form.
    pub min_lines: u32,
}

impl TsNormalizer {
    /// Builds a normalizer with size thresholds.
    #[must_use]
    pub const fn new(min_nodes: u32, min_lines: u32) -> Self {
        Self {
            min_nodes,
            min_lines,
        }
    }
}

impl LanguageNormalizer for TsNormalizer {
    fn normalize_file(
        &self,
        path: &Path,
        source: &str,
        next_id: &mut u64,
    ) -> Result<NormalizeOutcome, NormalizeError> {
        if file_is_ignored(source, MARKER) || is_declaration_file(path) {
            return Ok(NormalizeOutcome {
                forms: Vec::new(),
                warnings: Vec::new(),
            });
        }
        let parsed = parse_source(path, source)?;
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
            warnings
                .push("parse error in TypeScript source; extracted from partial CST".to_owned());
        }
        Ok(NormalizeOutcome { forms, warnings })
    }
}

/// Classifies production versus test from TypeScript path conventions.
#[must_use]
pub fn kind_from_path(path: &Path) -> FormKind {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    if name.contains(".test.") || name.contains(".spec.") {
        return FormKind::Test;
    }
    for component in path.components() {
        if component.as_os_str() == "__tests__" {
            return FormKind::Test;
        }
    }
    FormKind::Production
}

/// Declaration files end in `.d.ts` / `.d.mts` / `.d.cts` (last extension is still `ts`).
#[must_use]
pub fn is_declaration_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    name.ends_with(".d.ts") || name.ends_with(".d.mts") || name.ends_with(".d.cts")
}
