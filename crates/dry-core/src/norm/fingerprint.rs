//! Subtree fingerprinting over [`NormNode`] trees.
//!
//! Digests use a fixed FNV-1a 64-bit protocol so fingerprints are stable across
//! toolchains and CI runners. Birthday collisions on 64-bit digests are
//! theoretically possible and may cause rare false similarity; that risk is
//! accepted for this tool's local-analysis threat model.

use std::collections::BTreeMap;

use super::tree::NormNode;

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Fingerprint bag and node count for a normalized tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FingerprintResult {
    /// Subtree hash → multiplicity (bag / multiset).
    pub fingerprints: BTreeMap<u64, u32>,
    /// Total nodes in the tree.
    pub node_count: u32,
}

/// Hashes every subtree and returns the fingerprint bag.
#[must_use]
pub fn fingerprint_tree(root: &NormNode) -> FingerprintResult {
    let mut fingerprints = BTreeMap::new();
    let mut node_count = 0_u32;
    let _ = hash_node(root, &mut fingerprints, &mut node_count);
    FingerprintResult {
        fingerprints,
        node_count,
    }
}

fn hash_node(node: &NormNode, fingerprints: &mut BTreeMap<u64, u32>, node_count: &mut u32) -> u64 {
    *node_count = node_count.saturating_add(1);
    let mut child_hashes = Vec::with_capacity(node.children.len());
    for child in &node.children {
        child_hashes.push(hash_node(child, fingerprints, node_count));
    }
    let digest = hash_labeled_subtree(&node.label, &child_hashes);
    *fingerprints.entry(digest).or_default() += 1;
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
        let expected: BTreeMap<u64, u32> = [
            (0x9f82_3fc5_49e8_4070, 1),
            (0x9f85_a5c5_49eb_2399, 1),
            (0x6d21_5d42_1f39_7d61, 1),
        ]
        .into_iter()
        .collect();
        assert_eq!(result.fingerprints, expected);
    }

    #[test]
    fn repeated_identical_subtrees_increase_counts() {
        let leaf = NormNode::leaf("id0");
        let tree = NormNode::branch("block", vec![leaf.clone(), leaf]);
        let result = fingerprint_tree(&tree);
        let leaf_only = fingerprint_tree(&NormNode::leaf("id0"));
        let leaf_hash = leaf_only.fingerprints.keys().copied().next();
        assert_eq!(
            leaf_hash.and_then(|h| result.fingerprints.get(&h).copied()),
            Some(2)
        );
    }
}
