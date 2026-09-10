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
            if skip_near_miss_candidate(right_idx, left_idx, &mut seen) {
                continue;
            }
            best = consider_near_miss(left, remaining[right_idx], claimed, threshold, best);
        }
    }
    best
}

fn skip_near_miss_candidate(right_idx: usize, left_idx: usize, seen: &mut BTreeSet<usize>) -> bool {
    right_idx <= left_idx || !seen.insert(right_idx)
}

fn consider_near_miss<'a>(
    left: &'a NormalizedForm,
    right: &'a NormalizedForm,
    claimed: &BTreeSet<u64>,
    threshold: f64,
    best: Option<(&'a NormalizedForm, f64)>,
) -> Option<(&'a NormalizedForm, f64)> {
    scored_near_miss(left, right, claimed, threshold)
        .map(|score| prefer_better_match(best, right, score))
        .or(best)
}

fn scored_near_miss(
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

fn near_miss_eligible(
    left: &NormalizedForm,
    right: &NormalizedForm,
    claimed: &BTreeSet<u64>,
    threshold: f64,
) -> bool {
    !claimed.contains(&right.id)
        && left.kind == right.kind
        && within_jaccard_window(left.fingerprints.len(), right.fingerprints.len(), threshold)
}

fn partial_jaccard_score(score: f64, threshold: f64) -> Option<f64> {
    (score >= threshold && score < 1.0).then_some(score)
}

fn prefer_better_match<'a>(
    best: Option<(&'a NormalizedForm, f64)>,
    candidate: &'a NormalizedForm,
    score: f64,
) -> (&'a NormalizedForm, f64) {
    let Some((best_form, best_score)) = best else {
        return (candidate, score);
    };
    if is_better_match(score, best_score, candidate.id, best_form.id) {
        (candidate, score)
    } else {
        (best_form, best_score)
    }
}

fn is_better_match(score: f64, best_score: f64, candidate_id: u64, best_id: u64) -> bool {
    match score.total_cmp(&best_score) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => candidate_id < best_id,
        std::cmp::Ordering::Less => false,
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
mod tests;
