//! Normalized form IR consumed by the comparison engine.

use std::collections::BTreeMap;
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
    /// Count of structural nodes used for extract-time size filtering.
    pub node_count: u32,
    /// Structural fingerprint bag (subtree hash → multiplicity).
    pub fingerprints: BTreeMap<u64, u32>,
    /// Raw identifier spellings in normalization visit order; repeats included.
    pub ident_trace: Vec<String>,
}

impl NormalizedForm {
    /// Fold of fingerprint hash and multiplicity for exact-match bucketing.
    ///
    /// Mixes count so duplicate subtree hashes do not cancel, and uses an
    /// order-stable multiply-xor fold over the sorted bag to reduce structured
    /// XOR collisions (exactness remains guarded by bag equality).
    #[must_use]
    pub fn bucket_key(&self) -> u64 {
        const FOLD_PRIME: u64 = 0x0000_0100_0000_01b3;
        self.fingerprints.iter().fold(0_u64, |acc, (fp, count)| {
            let mixed = fp.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(u64::from(*count));
            acc.wrapping_mul(FOLD_PRIME) ^ mixed
        })
    }

    /// Total multiset size (`Σ` counts) used for Jaccard windowing.
    #[must_use]
    pub fn bag_size(&self) -> usize {
        self.fingerprints
            .values()
            .fold(0_usize, |acc, &count| acc.saturating_add(count as usize))
    }
}
