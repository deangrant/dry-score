//! Multiset Jaccard similarity over fingerprint bags.

use std::collections::BTreeMap;

/// Multiset Jaccard index: `Σ min(c_a,c_b) / Σ max(c_a,c_b)`.
///
/// Returns `0.0` when either bag is empty, including both empty.
#[must_use]
pub fn jaccard(left: &BTreeMap<u64, u32>, right: &BTreeMap<u64, u32>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let mut intersection = 0_u64;
    let mut union = 0_u64;
    for (key, &left_count) in left {
        let right_count = right.get(key).copied().unwrap_or(0);
        intersection = intersection.saturating_add(u64::from(left_count.min(right_count)));
        union = union.saturating_add(u64::from(left_count.max(right_count)));
    }
    for (key, &right_count) in right {
        if !left.contains_key(key) {
            union = union.saturating_add(u64::from(right_count));
        }
    }
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(pairs: &[(u64, u32)]) -> BTreeMap<u64, u32> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn identical_bags_score_one() {
        let set = bag(&[(1, 1), (2, 1), (3, 1)]);
        assert!((jaccard(&set, &set) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_scores_zero() {
        let empty = BTreeMap::new();
        let nonempty = bag(&[(1, 1)]);
        assert!((jaccard(&empty, &empty) - 0.0).abs() < f64::EPSILON);
        assert!((jaccard(&empty, &nonempty) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn partial_overlap() {
        let a = bag(&[(1, 1), (2, 1), (3, 1)]);
        let b = bag(&[(2, 1), (3, 1), (4, 1)]);
        let score = jaccard(&a, &b);
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn differing_multiplicity_scores_below_one() {
        let a = bag(&[(1, 2), (2, 1)]);
        let b = bag(&[(1, 1), (2, 1)]);
        let score = jaccard(&a, &b);
        assert!(score < 1.0);
        // min: 1+1=2, max: 2+1=3 → 2/3
        assert!((score - (2.0 / 3.0)).abs() < f64::EPSILON);
    }
}
