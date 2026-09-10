//! Comparison engine: exact buckets then inverted-index Jaccard near-miss.

mod classify;
mod jaccard;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::domain::{Finding, FormKind, FormMember, NormalizedForm};

#[doc(inline)]
pub use classify::{AUTO_REFACTOR_FLOOR, REVIEW_FIRST_FLOOR, classify};
#[doc(inline)]
pub use jaccard::jaccard;

/// Compares normalized forms and returns findings above `threshold`.
///
/// Exact fingerprint-set matches become n-ary findings at score `1.0`.
/// Remaining forms are matched with Jaccard via an inverted fingerprint index;
/// each form is claimed by at most one near-miss pair (greedy best match).
/// Production and test forms (`FormKind`) are never paired with each other.
#[must_use]
pub fn compare(forms: &[NormalizedForm], threshold: f64) -> Vec<Finding> {
    let mut claimed = BTreeSet::new();
    let mut findings = exact_bucket_findings(forms, &mut claimed, threshold);
    findings.extend(near_miss_findings(forms, &mut claimed, threshold));
    sort_findings(&mut findings);
    findings
}

fn exact_bucket_findings(
    forms: &[NormalizedForm],
    claimed: &mut BTreeSet<u64>,
    threshold: f64,
) -> Vec<Finding> {
    let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();
    for (idx, form) in forms.iter().enumerate() {
        if form.fingerprints.is_empty() {
            continue;
        }
        buckets.entry(form.bucket_key()).or_default().push(idx);
    }

    let mut findings = Vec::new();
    for indices in buckets.values() {
        if indices.len() < 2 {
            continue;
        }
        push_exact_clusters(forms, indices, claimed, threshold, &mut findings);
    }
    findings
}

fn push_exact_clusters(
    forms: &[NormalizedForm],
    indices: &[usize],
    claimed: &mut BTreeSet<u64>,
    threshold: f64,
    findings: &mut Vec<Finding>,
) {
    let mut by_set: BTreeMap<&BTreeSet<u64>, Vec<usize>> = BTreeMap::new();
    for &idx in indices {
        by_set.entry(&forms[idx].fingerprints).or_default().push(idx);
    }
    for group in by_set.values() {
        push_same_kind_clusters(forms, group, claimed, threshold, findings);
    }
}

fn push_same_kind_clusters(
    forms: &[NormalizedForm],
    group: &[usize],
    claimed: &mut BTreeSet<u64>,
    threshold: f64,
    findings: &mut Vec<Finding>,
) {
    let mut by_kind: BTreeMap<FormKind, Vec<usize>> = BTreeMap::new();
    for &idx in group {
        by_kind.entry(forms[idx].kind).or_default().push(idx);
    }
    for kind_group in by_kind.values() {
        push_cluster_if_pair(forms, kind_group, claimed, threshold, findings);
    }
}

fn push_cluster_if_pair(
    forms: &[NormalizedForm],
    group: &[usize],
    claimed: &mut BTreeSet<u64>,
    threshold: f64,
    findings: &mut Vec<Finding>,
) {
    if group.len() < 2 {
        return;
    }
    let members = members_from_indices(forms, group);
    let idents_identical = idents_match(forms, group);
    findings.push(Finding {
        clone_type: classify(1.0, idents_identical),
        tier: classify::tier_for(1.0, threshold),
        score: 1.0,
        members,
    });
    for &idx in group {
        claimed.insert(forms[idx].id);
    }
}

fn idents_match(forms: &[NormalizedForm], indices: &[usize]) -> bool {
    let Some(first) = indices.first() else {
        return true;
    };
    let probe = &forms[*first].ident_trace;
    indices.iter().all(|&idx| forms[idx].ident_trace == *probe)
}

fn near_miss_findings(
    forms: &[NormalizedForm],
    claimed: &mut BTreeSet<u64>,
    threshold: f64,
) -> Vec<Finding> {
    let remaining = unclaimed_sorted(forms, claimed);
    let index = build_fingerprint_index(&remaining);
    let mut findings = Vec::new();
    for (left_idx, left) in remaining.iter().enumerate() {
        if claimed.contains(&left.id) {
            continue;
        }
        let Some((right, score)) =
            best_near_miss(left, left_idx, &remaining, &index, claimed, threshold)
        else {
            continue;
        };
        findings.push(near_miss_finding(left, right, score, threshold));
        claimed.insert(left.id);
        claimed.insert(right.id);
    }
    findings
}

fn unclaimed_sorted<'a>(
    forms: &'a [NormalizedForm],
    claimed: &BTreeSet<u64>,
) -> Vec<&'a NormalizedForm> {
    let mut remaining: Vec<&NormalizedForm> = forms
        .iter()
        .filter(|f| !claimed.contains(&f.id) && !f.fingerprints.is_empty())
        .collect();
    remaining.sort_by_key(|f| (f.fingerprints.len(), f.id));
    remaining
}

fn build_fingerprint_index(remaining: &[&NormalizedForm]) -> HashMap<u64, Vec<usize>> {
    let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
    for (idx, form) in remaining.iter().enumerate() {
        for &fp in &form.fingerprints {
            index.entry(fp).or_default().push(idx);
        }
    }
    index
}

fn best_near_miss<'a>(
    left: &'a NormalizedForm,
    left_idx: usize,
    remaining: &[&'a NormalizedForm],
    index: &HashMap<u64, Vec<usize>>,
    claimed: &BTreeSet<u64>,
    threshold: f64,
) -> Option<(&'a NormalizedForm, f64)> {
    let mut seen = BTreeSet::new();
    let mut best: Option<(&NormalizedForm, f64)> = None;
    for &fp in &left.fingerprints {
        let Some(postings) = index.get(&fp) else {
            continue;
        };
        for &right_idx in postings {
            if right_idx <= left_idx || !seen.insert(right_idx) {
                continue;
            }
            let right = remaining[right_idx];
            if let Some(score) = scored_near_miss(left, right, claimed, threshold) {
                best = Some(prefer_better_match(best, right, score));
            }
        }
    }
    best
}

fn scored_near_miss(
    left: &NormalizedForm,
    right: &NormalizedForm,
    claimed: &BTreeSet<u64>,
    threshold: f64,
) -> Option<f64> {
    if claimed.contains(&right.id) || left.kind != right.kind {
        return None;
    }
    if !within_jaccard_window(left.fingerprints.len(), right.fingerprints.len(), threshold) {
        return None;
    }
    let score = jaccard(&left.fingerprints, &right.fingerprints);
    if score < threshold || score >= 1.0 {
        None
    } else {
        Some(score)
    }
}

fn prefer_better_match<'a>(
    best: Option<(&'a NormalizedForm, f64)>,
    candidate: &'a NormalizedForm,
    score: f64,
) -> (&'a NormalizedForm, f64) {
    let Some((best_form, best_score)) = best else {
        return (candidate, score);
    };
    let better = match score.total_cmp(&best_score) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => candidate.id < best_form.id,
        std::cmp::Ordering::Less => false,
    };
    if better {
        (candidate, score)
    } else {
        (best_form, best_score)
    }
}

fn near_miss_finding(
    left: &NormalizedForm,
    right: &NormalizedForm,
    score: f64,
    threshold: f64,
) -> Finding {
    Finding {
        clone_type: classify(score, false),
        tier: classify::tier_for(score, threshold),
        score,
        members: vec![
            FormMember::new(left.path.clone(), left.span, left.name.clone()),
            FormMember::new(right.path.clone(), right.span, right.name.clone()),
        ],
    }
}

fn within_jaccard_window(smaller: usize, larger: usize, threshold: f64) -> bool {
    if threshold <= 0.0 {
        return true;
    }
    let max_allowed = smaller as f64 / threshold;
    larger as f64 <= max_allowed
}

fn members_from_indices(forms: &[NormalizedForm], indices: &[usize]) -> Vec<FormMember> {
    let mut members: Vec<FormMember> = indices
        .iter()
        .map(|&i| FormMember::new(forms[i].path.clone(), forms[i].span, forms[i].name.clone()))
        .collect();
    members.sort_by(|a, b| (&a.path, a.start_line, &a.name).cmp(&(&b.path, b.start_line, &b.name)));
    members
}

fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| member_sort_key(a).cmp(&member_sort_key(b)))
    });
}

fn member_sort_key(finding: &Finding) -> (String, u32, String) {
    finding
        .members
        .first()
        .map(|m| (m.path.display().to_string(), m.start_line, m.name.clone()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FormKind, FormSpan};
    use std::path::PathBuf;

    fn form(id: u64, nodes: u32, fps: &[u64], idents: &[&str]) -> NormalizedForm {
        form_kind(id, nodes, fps, idents, FormKind::Production)
    }

    fn form_kind(
        id: u64,
        nodes: u32,
        fps: &[u64],
        idents: &[&str],
        kind: FormKind,
    ) -> NormalizedForm {
        NormalizedForm {
            id,
            name: format!("f{id}"),
            path: PathBuf::from("a.rs"),
            span: FormSpan::new(1, 10),
            kind,
            node_count: nodes,
            fingerprints: fps.iter().copied().collect(),
            ident_trace: idents.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn assert_compare(
        forms: &[NormalizedForm],
        threshold: f64,
        expected_type: crate::domain::CloneType,
        exact_score: bool,
    ) {
        let findings = compare(forms, threshold);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].clone_type, expected_type);
        if exact_score {
            assert!((findings[0].score - 1.0).abs() < f64::EPSILON);
        } else {
            assert!(findings[0].score < 1.0);
            assert!(findings[0].score >= threshold);
        }
    }

    #[test]
    fn exact_bucket_clusters_identical_sets() {
        // dry-rs:ignore. Compact compare harness for exact buckets.
        let forms = [
            form(1, 10, &[1, 2, 3], &["x"]),
            form(2, 10, &[1, 2, 3], &["y"]),
        ];
        assert_compare(&forms, 0.85, crate::domain::CloneType::Type2, true);
    }

    #[test]
    fn empty_fingerprints_never_match() {
        let forms = vec![form(1, 1, &[], &[]), form(2, 1, &[], &[])];
        assert!(compare(&forms, 0.5).is_empty());
    }

    #[test]
    fn near_miss_emits_type_three() {
        // dry-rs:ignore. Compact compare harness for near-miss Jaccard.
        let left = form(1, 5, &[1, 2, 3, 4, 5], &["a"]);
        let right = form(2, 5, &[1, 2, 3, 4, 9], &["a"]);
        let findings = compare(&[left, right], 0.5);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].clone_type, crate::domain::CloneType::Type3);
        assert!(findings[0].score < 1.0);
    }

    #[test]
    fn singleton_bucket_is_ignored() {
        let forms = [form(1, 10, &[1, 2], &["x"])];
        assert!(compare(&forms, 0.5).is_empty());
    }

    #[test]
    fn near_miss_skips_below_threshold() {
        let left = form(1, 4, &[1, 2, 3, 4], &["a"]);
        let right = form(2, 4, &[9, 8, 7, 6], &["a"]);
        assert!(compare(&[left, right], 0.9).is_empty());
    }

    #[test]
    fn zero_threshold_keeps_window_open() {
        assert!(within_jaccard_window(1, 100, 0.0));
    }

    #[test]
    fn sort_orders_by_score_then_member() {
        let mut findings = vec![
            Finding {
                clone_type: crate::domain::CloneType::Type3,
                tier: crate::domain::Tier::Advisory,
                score: 0.8,
                members: vec![FormMember::new(
                    PathBuf::from("b.rs"),
                    FormSpan::new(1, 2),
                    "b".to_owned(),
                )],
            },
            Finding {
                clone_type: crate::domain::CloneType::Type3,
                tier: crate::domain::Tier::Advisory,
                score: 0.9,
                members: vec![FormMember::new(
                    PathBuf::from("a.rs"),
                    FormSpan::new(1, 2),
                    "a".to_owned(),
                )],
            },
        ];
        sort_findings(&mut findings);
        assert!((findings[0].score - 0.9).abs() < f64::EPSILON);
        let empty = Finding {
            clone_type: crate::domain::CloneType::Type3,
            tier: crate::domain::Tier::Advisory,
            score: 0.1,
            members: Vec::new(),
        };
        assert_eq!(member_sort_key(&empty), (String::new(), 0, String::new()));
    }

    #[test]
    fn idents_match_empty_indices() {
        assert!(idents_match(&[], &[]));
    }

    #[test]
    fn colliding_bucket_keys_without_identical_sets() {
        // 1 ^ 2 == 3, so these share a bucket key but not a fingerprint set.
        let forms = [form(1, 10, &[1, 2], &["x"]), form(2, 10, &[3], &["y"])];
        assert!(compare(&forms, 0.99).is_empty());
    }

    #[test]
    fn scored_near_miss_rejects_exact_and_out_of_window() {
        let twin = form(1, 5, &[1, 2, 3, 4, 5], &["a"]);
        let same = form(2, 5, &[1, 2, 3, 4, 5], &["b"]);
        let claimed = BTreeSet::new();
        assert!(scored_near_miss(&twin, &same, &claimed, 0.5).is_none());
        // Set sizes 2 vs 20 fall outside a 0.9 Jaccard upper bound.
        let small = form(3, 100, &[1, 2], &["a"]);
        let large_fps: Vec<u64> = (1..=20).collect();
        let large = form(4, 10, &large_fps, &["a"]);
        assert!(scored_near_miss(&small, &large, &claimed, 0.9).is_none());
    }

    #[test]
    fn near_miss_skips_out_of_window_shared_fingerprint() {
        let large_fps: Vec<u64> = (1..=40).collect();
        let forms = [
            form(1, 50, &[1, 9], &["a"]),
            form(2, 10, &large_fps, &["a"]),
        ];
        assert!(compare(&forms, 0.9).is_empty());
    }

    #[test]
    fn near_miss_claims_clique_once() {
        let forms = [
            form(1, 4, &[1, 2, 3, 4], &["a"]),
            form(2, 4, &[1, 2, 3, 5], &["a"]),
            form(3, 4, &[1, 2, 3, 6], &["a"]),
        ];
        let findings = compare(&forms, 0.5);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].members.len(), 2);
        assert_eq!(findings[0].clone_type, crate::domain::CloneType::Type3);
    }

    #[test]
    fn near_miss_reports_disjoint_pairs() {
        let forms = [
            form(1, 4, &[1, 2, 3, 4], &["a"]),
            form(2, 4, &[1, 2, 3, 5], &["a"]),
            form(3, 4, &[10, 11, 12, 13], &["a"]),
            form(4, 4, &[10, 11, 12, 14], &["a"]),
        ];
        let findings = compare(&forms, 0.5);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().all(|f| f.clone_type == crate::domain::CloneType::Type3));
    }

    #[test]
    fn near_miss_no_shared_fingerprints_never_pairs() {
        let left = form(1, 4, &[1, 2, 3, 4], &["a"]);
        let right = form(2, 4, &[9, 8, 7, 6], &["a"]);
        assert!(compare(&[left, right], 0.1).is_empty());
    }

    #[test]
    fn disparate_node_count_still_finds_type_three() {
        // Identical set sizes and high overlap; node_count must not prune.
        let left = form(1, 100, &[1, 2, 3, 4, 5], &["a"]);
        let right = form(2, 10, &[1, 2, 3, 4, 9], &["a"]);
        let findings = compare(&[left, right], 0.5);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].clone_type, crate::domain::CloneType::Type3);
    }

    #[test]
    fn production_and_test_are_not_paired() {
        let prod = form_kind(1, 10, &[1, 2, 3], &["x"], FormKind::Production);
        let test = form_kind(2, 10, &[1, 2, 3], &["x"], FormKind::Test);
        assert!(compare(&[prod, test], 0.5).is_empty());
    }

    #[test]
    fn same_kind_test_twins_still_match() {
        let forms = [
            form_kind(1, 10, &[1, 2, 3], &["x"], FormKind::Test),
            form_kind(2, 10, &[1, 2, 3], &["y"], FormKind::Test),
        ];
        assert_compare(&forms, 0.85, crate::domain::CloneType::Type2, true);
    }
}
