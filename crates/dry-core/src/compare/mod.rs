//! Comparison engine: exact buckets then inverted-index Jaccard near-miss.
//!
//! Near-miss candidates use a DF-ordered Jaccard occurrence prefix so common
//! leaf digests do not dominate inverted-index probing.

mod classify;
mod jaccard;
mod near_miss;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::domain::{Finding, FormKind, FormMember, NormalizedForm};

#[doc(inline)]
pub use classify::{AUTO_REFACTOR_FLOOR, REVIEW_FIRST_FLOOR, classify};
#[doc(inline)]
pub use jaccard::jaccard;

use near_miss::collect_near_miss_edges;

#[cfg(test)]
use near_miss::{fingerprint_df_for_test, prefix_keys, scored_near_miss};

/// Compares normalized forms and returns findings above `threshold`.
///
/// Exact fingerprint-bag matches become n-ary findings at score `1.0`.
/// Remaining forms are linked by multiset Jaccard via an inverted fingerprint
/// index (DF-ordered occurrence prefix for candidates) into multi-member
/// Type-3 components (connected components; score is the minimum pairwise
/// Jaccard among members). Components that are not threshold-closed split into
/// exclusive pair findings. Production and test forms (`FormKind`) never pair.
#[must_use]
pub fn compare(forms: &[NormalizedForm], threshold: f64) -> Vec<Finding> {
    let mut claimed = BTreeSet::new();
    let mut findings = exact_bucket_findings(forms, &mut claimed);
    findings.extend(near_miss_findings(forms, &mut claimed, threshold));
    sort_findings(&mut findings);
    findings
}

fn exact_bucket_findings(forms: &[NormalizedForm], claimed: &mut BTreeSet<u64>) -> Vec<Finding> {
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
        push_exact_clusters(forms, indices, claimed, &mut findings);
    }
    findings
}

fn push_exact_clusters(
    forms: &[NormalizedForm],
    indices: &[usize],
    claimed: &mut BTreeSet<u64>,
    findings: &mut Vec<Finding>,
) {
    let mut by_set: BTreeMap<&BTreeMap<u64, u32>, Vec<usize>> = BTreeMap::new();
    for &idx in indices {
        by_set.entry(&forms[idx].fingerprints).or_default().push(idx);
    }
    for group in by_set.values() {
        push_same_kind_clusters(forms, group, claimed, findings);
    }
}

fn push_same_kind_clusters(
    forms: &[NormalizedForm],
    group: &[usize],
    claimed: &mut BTreeSet<u64>,
    findings: &mut Vec<Finding>,
) {
    let mut by_kind: BTreeMap<FormKind, Vec<usize>> = BTreeMap::new();
    for &idx in group {
        by_kind.entry(forms[idx].kind).or_default().push(idx);
    }
    for kind_group in by_kind.values() {
        push_cluster_if_pair(forms, kind_group, claimed, findings);
    }
}

fn push_cluster_if_pair(
    forms: &[NormalizedForm],
    group: &[usize],
    claimed: &mut BTreeSet<u64>,
    findings: &mut Vec<Finding>,
) {
    if group.len() < 2 {
        return;
    }
    let (identical, leftovers) = partition_by_idents(forms, group);
    for ident_group in &identical {
        push_exact_finding(forms, ident_group, claimed, true, findings);
    }
    // Renamed leftovers share the bag with Type-1 siblings; emit Type-2 for the
    // full kind-group so a singleton rename is not dropped after Type-1 claims.
    if !leftovers.is_empty() {
        push_exact_finding(forms, group, claimed, false, findings);
    }
}

fn partition_by_idents(forms: &[NormalizedForm], group: &[usize]) -> (Vec<Vec<usize>>, Vec<usize>) {
    let mut by_idents: BTreeMap<&[String], Vec<usize>> = BTreeMap::new();
    for &idx in group {
        by_idents.entry(forms[idx].ident_trace.as_slice()).or_default().push(idx);
    }
    let mut identical = Vec::new();
    let mut leftovers = Vec::new();
    for ident_group in by_idents.into_values() {
        if ident_group.len() >= 2 {
            identical.push(ident_group);
        } else {
            leftovers.extend(ident_group);
        }
    }
    (identical, leftovers)
}

fn push_exact_finding(
    forms: &[NormalizedForm],
    group: &[usize],
    claimed: &mut BTreeSet<u64>,
    idents_identical: bool,
    findings: &mut Vec<Finding>,
) {
    findings.push(Finding {
        clone_type: classify(1.0, idents_identical),
        tier: classify::tier_for(1.0),
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
    for member_idxs in components {
        if let Some(min_pair) = component_closed_min(&member_idxs, &edges) {
            findings.push(near_miss_component_finding(
                &remaining,
                &member_idxs,
                min_pair,
            ));
            for &idx in &member_idxs {
                claimed.insert(remaining[idx].id);
            }
        } else {
            emit_greedy_pair_findings(&remaining, &member_idxs, &edges, claimed, &mut findings);
        }
    }
    findings
}

fn unclaimed_sorted<'a>(
    forms: &'a [NormalizedForm],
    claimed: &BTreeSet<u64>,
) -> Vec<&'a NormalizedForm> {
    // dry-rs:ignore. Collect-then-sort helper; parallel with members_from_indices.
    let mut remaining: Vec<&NormalizedForm> = forms
        .iter()
        .filter(|f| !claimed.contains(&f.id) && !f.fingerprints.is_empty())
        .collect();
    remaining.sort_by_key(|f| (f.bag_size(), f.id));
    remaining
}

fn near_miss_components(n: usize, edges: &[(usize, usize, f64)]) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..n).collect();
    for &(a, b, _) in edges {
        let ra = find_root(&mut parent, a);
        let rb = find_root(&mut parent, b);
        if ra != rb {
            parent[rb] = ra;
        }
    }
    group_components(n, &mut parent)
}

fn find_root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn group_components(n: usize, parent: &mut [usize]) -> Vec<Vec<usize>> {
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        groups.entry(find_root(parent, i)).or_default().push(i);
    }
    groups.into_values().filter(|members| members.len() >= 2).collect()
}

/// Threshold-closed when the component is a clique in the ≥-threshold edge
/// graph (DF-prefix index already scored every such pair). Score is the min
/// edge weight.
fn component_closed_min(members: &[usize], edges: &[(usize, usize, f64)]) -> Option<f64> {
    let member_set: BTreeSet<usize> = members.iter().copied().collect();
    let mut min_score = 1.0_f64;
    let mut edge_count = 0_usize;
    for &(left, right, score) in edges {
        if member_set.contains(&left) && member_set.contains(&right) {
            edge_count = edge_count.saturating_add(1);
            min_score = min_score.min(score);
        }
    }
    let k = members.len();
    let expected = k.saturating_mul(k.saturating_sub(1)) / 2;
    (edge_count == expected).then_some(min_score)
}

fn emit_greedy_pair_findings(
    remaining: &[&NormalizedForm],
    member_idxs: &[usize],
    edges: &[(usize, usize, f64)],
    claimed: &mut BTreeSet<u64>,
    findings: &mut Vec<Finding>,
) {
    let member_set: BTreeSet<usize> = member_idxs.iter().copied().collect();
    let mut component_edges: Vec<(usize, usize, f64)> = edges
        .iter()
        .copied()
        .filter(|(a, b, _)| member_set.contains(a) && member_set.contains(b))
        .collect();
    component_edges.sort_by(|left, right| right.2.total_cmp(&left.2));
    let mut used = BTreeSet::new();
    for (a, b, score) in component_edges {
        if used.contains(&a) || used.contains(&b) {
            continue;
        }
        used.insert(a);
        used.insert(b);
        findings.push(near_miss_component_finding(remaining, &[a, b], score));
        claimed.insert(remaining[a].id);
        claimed.insert(remaining[b].id);
    }
}

fn near_miss_component_finding(
    remaining: &[&NormalizedForm],
    member_idxs: &[usize],
    score: f64,
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
        tier: classify::tier_for(score),
        score,
        members,
    }
}

pub(super) fn within_jaccard_window(left_len: usize, right_len: usize, threshold: f64) -> bool {
    if threshold <= 0.0 {
        return true;
    }
    let smaller = left_len.min(right_len);
    let larger = left_len.max(right_len);
    let max_allowed = smaller as f64 / threshold;
    larger as f64 <= max_allowed
}

fn members_from_indices(forms: &[NormalizedForm], indices: &[usize]) -> Vec<FormMember> {
    // dry-rs:ignore. Collect-then-sort helper; parallel with unclaimed_sorted.
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
    // dry-rs:ignore. Thin map-or-default; parallel shape with path_segment_leaves.
    finding
        .members
        .first()
        .map(|m| (m.path.display().to_string(), m.start_line, m.name.clone()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
