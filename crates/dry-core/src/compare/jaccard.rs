//! Jaccard similarity over fingerprint sets.

use std::collections::BTreeSet;

/// Jaccard index of two fingerprint sets.
///
/// Returns `0.0` when either set is empty, including both empty.
#[must_use]
pub fn jaccard(left: &BTreeSet<u64>, right: &BTreeSet<u64>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let intersection = left.intersection(right).count();
    let union = left.union(right).count().max(1);
    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_sets_score_one() {
        let set: BTreeSet<u64> = [1, 2, 3].into_iter().collect();
        assert!((jaccard(&set, &set) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_scores_zero() {
        let empty = BTreeSet::new();
        let nonempty: BTreeSet<u64> = std::iter::once(1).collect();
        assert!((jaccard(&empty, &empty) - 0.0).abs() < f64::EPSILON);
        assert!((jaccard(&empty, &nonempty) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn partial_overlap() {
        let a: BTreeSet<u64> = [1, 2, 3].into_iter().collect();
        let b: BTreeSet<u64> = [2, 3, 4].into_iter().collect();
        let score = jaccard(&a, &b);
        assert!((score - 0.5).abs() < f64::EPSILON);
    }
}
