//! Near-miss candidate generation via a DF-ordered Jaccard prefix index.

use std::collections::{BTreeSet, HashMap};

use crate::domain::NormalizedForm;

use super::jaccard::jaccard;
use super::within_jaccard_window;

/// Collects near-miss edges among `remaining` forms at or above `threshold`.
///
/// Candidate generation indexes and probes only each form's DF-ordered
/// occurrence prefix so ubiquitous leaf digests do not fan out to all pairs.
/// Scoring still uses full-bag multiset Jaccard.
pub(super) fn collect_near_miss_edges(
    remaining: &[&NormalizedForm],
    threshold: f64,
) -> Vec<(usize, usize, f64)> {
    let df = fingerprint_df(remaining);
    let index = build_fingerprint_index(remaining, &df, threshold);
    let claimed = BTreeSet::new();
    let mut edges = Vec::new();
    let mut seen_pairs = BTreeSet::new();
    for (left_idx, left) in remaining.iter().enumerate() {
        for fp in prefix_keys(left, &df, threshold) {
            let postings = index.get(&fp).map_or(&[][..], Vec::as_slice);
            for &right_idx in postings {
                if let Some(score) = edge_score_if_new(
                    remaining,
                    left_idx,
                    left,
                    right_idx,
                    &claimed,
                    threshold,
                    &mut seen_pairs,
                ) {
                    edges.push((left_idx, right_idx, score));
                }
            }
        }
    }
    edges
}

fn fingerprint_df(remaining: &[&NormalizedForm]) -> HashMap<u64, usize> {
    let mut df = HashMap::new();
    for form in remaining {
        for &fp in form.fingerprints.keys() {
            *df.entry(fp).or_default() += 1;
        }
    }
    df
}

#[cfg(test)]
pub(super) fn fingerprint_df_for_test(remaining: &[&NormalizedForm]) -> HashMap<u64, usize> {
    fingerprint_df(remaining)
}

fn build_fingerprint_index(
    remaining: &[&NormalizedForm],
    df: &HashMap<u64, usize>,
    threshold: f64,
) -> HashMap<u64, Vec<usize>> {
    let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
    for (idx, form) in remaining.iter().enumerate() {
        for fp in prefix_keys(form, df, threshold) {
            index.entry(fp).or_default().push(idx);
        }
    }
    index
}

/// Rarest keys whose multiplicities cover the Jaccard occurrence prefix mass.
#[must_use]
pub(super) fn prefix_keys(
    form: &NormalizedForm,
    df: &HashMap<u64, usize>,
    threshold: f64,
) -> Vec<u64> {
    let bag_size = form.bag_size();
    if form.fingerprints.is_empty() || bag_size == 0 {
        return Vec::new();
    }
    if threshold <= 0.0 {
        return form.fingerprints.keys().copied().collect();
    }
    let need = prefix_mass(bag_size, threshold);
    let keys = keys_sorted_by_df(form, df);
    accumulate_prefix_keys(form, &keys, need)
}

fn keys_sorted_by_df(form: &NormalizedForm, df: &HashMap<u64, usize>) -> Vec<u64> {
    let mut keys: Vec<u64> = form.fingerprints.keys().copied().collect();
    keys.sort_by(|&left, &right| {
        let left_df = df.get(&left).copied().unwrap_or(0);
        let right_df = df.get(&right).copied().unwrap_or(0);
        left_df.cmp(&right_df).then_with(|| left.cmp(&right))
    });
    keys
}

fn accumulate_prefix_keys(form: &NormalizedForm, keys: &[u64], need: usize) -> Vec<u64> {
    let mut mass = 0_usize;
    let mut out = Vec::new();
    for &fp in keys {
        if mass >= need {
            break;
        }
        let count = usize::try_from(*form.fingerprints.get(&fp).unwrap_or(&0)).unwrap_or(0);
        out.push(fp);
        mass = mass.saturating_add(count);
    }
    out
}

fn prefix_mass(bag_size: usize, threshold: f64) -> usize {
    if threshold <= 0.0 || bag_size == 0 {
        return bag_size;
    }
    let ceil_t = (threshold * bag_size as f64).ceil() as usize;
    bag_size.saturating_sub(ceil_t).saturating_add(1)
}

fn edge_score_if_new(
    remaining: &[&NormalizedForm],
    left_idx: usize,
    left: &NormalizedForm,
    right_idx: usize,
    claimed: &BTreeSet<u64>,
    threshold: f64,
    seen_pairs: &mut BTreeSet<(usize, usize)>,
) -> Option<f64> {
    if right_idx <= left_idx {
        return None;
    }
    let right = remaining[right_idx];
    if !near_miss_eligible(left, right, claimed, threshold) {
        return None;
    }
    if !seen_pairs.insert((left_idx, right_idx)) {
        return None;
    }
    partial_jaccard_score(jaccard(&left.fingerprints, &right.fingerprints), threshold)
}

fn near_miss_eligible(
    left: &NormalizedForm,
    right: &NormalizedForm,
    claimed: &BTreeSet<u64>,
    threshold: f64,
) -> bool {
    !claimed.contains(&right.id)
        && left.kind == right.kind
        && within_jaccard_window(left.bag_size(), right.bag_size(), threshold)
}

fn partial_jaccard_score(score: f64, threshold: f64) -> Option<f64> {
    (score >= threshold && score < 1.0).then_some(score)
}

#[cfg(test)]
pub(super) fn scored_near_miss(
    left: &NormalizedForm,
    right: &NormalizedForm,
    claimed: &BTreeSet<u64>,
    threshold: f64,
) -> Option<f64> {
    if !near_miss_eligible(left, right, claimed, threshold) {
        return None;
    }
    partial_jaccard_score(jaccard(&left.fingerprints, &right.fingerprints), threshold)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FormKind, FormSpan, NormalizedForm};
    use std::path::PathBuf;

    fn form_bag(id: u64, pairs: &[(u64, u32)]) -> NormalizedForm {
        NormalizedForm {
            id,
            name: format!("f{id}"),
            path: PathBuf::from("a.rs"),
            span: FormSpan::new(1, 10),
            kind: FormKind::Production,
            node_count: 10,
            fingerprints: pairs.iter().copied().collect(),
            ident_trace: vec!["a".to_owned()],
        }
    }

    #[test]
    fn prefix_keys_prefer_low_df_and_cover_mass() {
        let forms = [
            form_bag(1, &[(1, 94), (10, 3), (11, 3)]),
            form_bag(2, &[(1, 94), (20, 3), (21, 3)]),
            form_bag(3, &[(1, 94), (30, 3), (31, 3)]),
        ];
        let refs: Vec<&NormalizedForm> = forms.iter().collect();
        let df = fingerprint_df(&refs);
        assert_eq!(df.get(&1), Some(&3));
        assert_eq!(df.get(&10), Some(&1));
        let keys = prefix_keys(&forms[0], &df, 0.85);
        // bag_size 100 → prefix_mass 16; rare keys 10+11 first (3+3), then common.
        assert!(keys.starts_with(&[10, 11]));
        assert!(keys.contains(&1));
        let mass: u32 =
            keys.iter().map(|fp| forms[0].fingerprints.get(fp).copied().unwrap_or(0)).sum();
        assert!(mass as usize >= prefix_mass(100, 0.85));
    }
}
