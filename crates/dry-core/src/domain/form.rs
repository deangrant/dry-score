//! Normalized form IR consumed by the comparison engine.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{FormKind, FormSpan};

/// Language-agnostic structural unit ready for comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedForm {
    /// Stable identity within one analysis run.
    pub id: u64,
    /// Human-readable name (function or block label).
    pub name: String,
    /// Absolute or repo-relative path to the source file.
    pub path: PathBuf,
    /// Line span of the form in the source file.
    pub span: FormSpan,
    /// Production versus test classification.
    pub kind: FormKind,
    /// Count of structural nodes used for window pruning.
    pub node_count: u32,
    /// Structural fingerprint set (subtree hashes).
    pub fingerprints: BTreeSet<u64>,
    /// Ordered raw identifiers encountered during normalization.
    pub ident_trace: Vec<String>,
}

impl NormalizedForm {
    /// XOR-fold of fingerprint elements for exact-match bucketing.
    #[must_use]
    pub fn bucket_key(&self) -> u64 {
        self.fingerprints.iter().fold(0_u64, |acc, fp| acc ^ fp)
    }
}
