//! Subtree fingerprinting over [`NormNode`] trees.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use super::tree::NormNode;

/// Fingerprint set and node count for a normalized tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FingerprintResult {
    /// All subtree hashes.
    pub fingerprints: BTreeSet<u64>,
    /// Total nodes in the tree.
    pub node_count: u32,
}

/// Hashes every subtree and returns the fingerprint set.
#[must_use]
pub fn fingerprint_tree(root: &NormNode) -> FingerprintResult {
    let mut fingerprints = BTreeSet::new();
    let mut node_count = 0_u32;
    let _ = hash_node(root, &mut fingerprints, &mut node_count);
    FingerprintResult {
        fingerprints,
        node_count,
    }
}

fn hash_node(node: &NormNode, fingerprints: &mut BTreeSet<u64>, node_count: &mut u32) -> u64 {
    *node_count = node_count.saturating_add(1);
    let mut child_hashes = Vec::with_capacity(node.children.len());
    for child in &node.children {
        child_hashes.push(hash_node(child, fingerprints, node_count));
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    node.label.hash(&mut hasher);
    for child_hash in child_hashes {
        child_hash.hash(&mut hasher);
    }
    let digest = hasher.finish();
    fingerprints.insert(digest);
    digest
}
