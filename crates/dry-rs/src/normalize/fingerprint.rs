//! Subtree fingerprinting over [`NormNode`] trees.
//!
//! Digests use a fixed FNV-1a 64-bit protocol so fingerprints are stable across
//! Rust toolchains and CI runners.

use std::collections::BTreeSet;

use super::tree::NormNode;

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

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
    let digest = hash_labeled_subtree(&node.label, &child_hashes);
    fingerprints.insert(digest);
    digest
}

/// Hashes `label` bytes, a `0` separator, then each child digest as LE `u64`.
fn hash_labeled_subtree(label: &str, child_hashes: &[u64]) -> u64 {
    let mut state = FNV_OFFSET;
    for byte in label.as_bytes() {
        state = fnv1a_byte(state, *byte);
    }
    state = fnv1a_byte(state, 0);
    for child in child_hashes {
        for byte in child.to_le_bytes() {
            state = fnv1a_byte(state, byte);
        }
    }
    state
}

fn fnv1a_byte(state: u64, byte: u8) -> u64 {
    (state ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_fingerprint_is_stable() {
        let tree = NormNode::branch(
            "binary:add",
            vec![NormNode::leaf("id0"), NormNode::leaf("id1")],
        );
        let result = fingerprint_tree(&tree);
        assert_eq!(result.node_count, 3);
        let expected: BTreeSet<u64> = [
            0x9f82_3fc5_49e8_4070,
            0x9f85_a5c5_49eb_2399,
            0x6d21_5d42_1f39_7d61,
        ]
        .into_iter()
        .collect();
        assert_eq!(result.fingerprints, expected);
    }
}
