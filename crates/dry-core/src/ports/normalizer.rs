//! [`LanguageNormalizer`] port for language-specific AST adapters.

use std::fmt;
use std::path::Path;

use crate::domain::NormalizedForm;

/// Error returned when a file cannot be normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizeError {
    /// Human-readable explanation.
    pub message: String,
}

impl NormalizeError {
    /// Builds an error from a displayable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for NormalizeError {}

/// Successful normalize result for one source file.
///
/// An empty `forms` vector is a valid success: the file was readable and either
/// contained no qualifying forms or opted out via a file-level suppress marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizeOutcome {
    /// Forms extracted from the file (already size-filtered by the adapter).
    pub forms: Vec<NormalizedForm>,
    /// Non-fatal adapter warnings (for example partial CST recovery).
    pub warnings: Vec<String>,
}

/// Language-specific normalizer that emits [`NormalizedForm`] values.
///
/// Adapters own parsing, rename-invariant normalization, size filtering, and
/// suppress-marker handling. Discovery (which paths to open) stays with the
/// caller via config-driven walk options.
pub trait LanguageNormalizer {
    /// Parses and normalizes one source file into comparable forms.
    ///
    /// Contracts:
    /// - Parse failure returns [`NormalizeError`].
    /// - File-level suppress returns `Ok` with an empty `forms` vector.
    /// - Emitted forms already satisfy the adapter's size thresholds.
    /// - Each emitted form receives a unique `id` by advancing `next_id`
    ///   monotonically (caller owns the counter across files).
    ///
    /// # Errors
    ///
    /// Returns [`NormalizeError`] when the source cannot be parsed.
    fn normalize_file(
        &self,
        path: &Path,
        source: &str,
        next_id: &mut u64,
    ) -> Result<NormalizeOutcome, NormalizeError>;
}
