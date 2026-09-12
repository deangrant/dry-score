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
/// Exact fingerprint-bag matches become n-ary findings at score `1.0`.
/// Remaining forms are linked by multiset Jaccard via an inverted fingerprint
/// index into multi-member Type-3 components (connected components; score is
/// the minimum edge Jaccard). Production and test forms (`FormKind`) never pair.
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
    let mut by_set: BTreeMap<&BTreeMap<u64, u32>, Vec<usize>> = BTreeMap::new();
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
    let mut by_idents: BTreeMap<&[String], Vec<usize>> = BTreeMap::new();
    for &idx in group {
        by_idents.entry(forms[idx].ident_trace.as_slice()).or_default().push(idx);
    }
    let mut leftovers = Vec::new();
    for ident_group in by_idents.values() {
        if ident_group.len() >= 2 {
            push_exact_finding(forms, ident_group, claimed, threshold, true, findings);
        } else {
            leftovers.extend(ident_group.iter().copied());
        }
    }
    if leftovers.len() >= 2 {
        push_exact_finding(forms, &leftovers, claimed, threshold, false, findings);
    }
}

fn push_exact_finding(
    forms: &[NormalizedForm],
    group: &[usize],
    claimed: &mut BTreeSet<u64>,
    threshold: f64,
    idents_identical: bool,
    findings: &mut Vec<Finding>,
) {
    findings.push(Finding {
        clone_type: classify(1.0, idents_identical),
        tier: classify::tier_for(1.0, threshold),
        score: 1.0,
        members: members_from_indices(forms, group),
    });
    for &idx in group {
        claimed.insert(forms[idx].id);
    }
}

#[cfg(test)]
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
    let edges = collect_near_miss_edges(&remaining, threshold);
    let components = near_miss_components(remaining.len(), &edges);
    let mut findings = Vec::new();
    for (member_idxs, score) in components {
        if member_idxs.len() < 2 {
            continue;
        }
        findings.push(near_miss_component_finding(
            &remaining,
            &member_idxs,
            score,
            threshold,
        ));
        for &idx in &member_idxs {
            claimed.insert(remaining[idx].id);
        }
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
    remaining.sort_by_key(|f| (f.bag_size(), f.id));
    remaining
}

fn build_fingerprint_index(remaining: &[&NormalizedForm]) -> HashMap<u64, Vec<usize>> {
    let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
    for (idx, form) in remaining.iter().enumerate() {
        for &fp in form.fingerprints.keys() {
            index.entry(fp).or_default().push(idx);
        }
    }
    index
}

fn collect_near_miss_edges(
    remaining: &[&NormalizedForm],
    threshold: f64,
) -> Vec<(usize, usize, f64)> {
    let index = build_fingerprint_index(remaining);
    let claimed = BTreeSet::new();
    let mut edges = Vec::new();
    let mut seen_pairs = BTreeSet::new();
    for (left_idx, left) in remaining.iter().enumerate() {
        for &fp in left.fingerprints.keys() {
            let Some(postings) = index.get(&fp) else {
                continue;
            };
            for &right_idx in postings {
                if right_idx <= left_idx || !seen_pairs.insert((left_idx, right_idx)) {
                    continue;
                }
                let right = remaining[right_idx];
                let Some(score) = scored_near_miss(left, right, &claimed, threshold) else {
                    continue;
                };
                edges.push((left_idx, right_idx, score));
            }
        }
    }
    edges
}

fn near_miss_components(n: usize, edges: &[(usize, usize, f64)]) -> Vec<(Vec<usize>, f64)> {
    let mut parent: Vec<usize> = (0..n).collect();
    let mut min_edge: Vec<Option<f64>> = vec![None; n];
    for &(a, b, score) in edges {
        let ra = find_root(&mut parent, a);
        let rb = find_root(&mut parent, b);
        let merged = [min_edge[ra], min_edge[rb], Some(score)]
            .into_iter()
            .flatten()
            .fold(score, f64::min);
        if ra != rb {
            parent[rb] = ra;
            min_edge[rb] = None;
        }
        min_edge[ra] = Some(merged);
    }
    group_components(n, &mut parent, &min_edge)
}

fn find_root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn group_components(
    n: usize,
    parent: &mut [usize],
    min_edge: &[Option<f64>],
) -> Vec<(Vec<usize>, f64)> {
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        groups.entry(find_root(parent, i)).or_default().push(i);
    }
    groups
        .into_iter()
        .filter_map(|(root, members)| {
            let score = min_edge[root]?;
            (members.len() >= 2).then_some((members, score))
        })
        .collect()
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
        && within_jaccard_window(left.bag_size(), right.bag_size(), threshold)
}

fn partial_jaccard_score(score: f64, threshold: f64) -> Option<f64> {
    (score >= threshold && score < 1.0).then_some(score)
}

fn near_miss_component_finding(
    remaining: &[&NormalizedForm],
    member_idxs: &[usize],
    score: f64,
    threshold: f64,
) -> Finding {
    let mut indices: Vec<usize> = member_idxs.to_vec();
    indices.sort_unstable();
    let mut members: Vec<FormMember> = indices
        .iter()
        .map(|&i| {
            FormMember::new(
                remaining[i].path.clone(),
                remaining[i].span,
                remaining[i].name.clone(),
            )
        })
        .collect();
    members.sort_by(|a, b| (&a.path, a.start_line, &a.name).cmp(&(&b.path, b.start_line, &b.name)));
    Finding {
        clone_type: classify(score, false),
        tier: classify::tier_for(score, threshold),
        score,
        members,
    }
}

fn within_jaccard_window(left_len: usize, right_len: usize, threshold: f64) -> bool {
    if threshold <= 0.0 {
        return true;
    }
    let smaller = left_len.min(right_len);
    let larger = left_len.max(right_len);
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
