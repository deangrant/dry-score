use super::*;
use crate::domain::{FormKind, FormSpan};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn bag(fps: &[u64]) -> BTreeMap<u64, u32> {
    let mut map = BTreeMap::new();
    for &fp in fps {
        *map.entry(fp).or_default() += 1;
    }
    map
}

fn form(id: u64, nodes: u32, fps: &[u64], idents: &[&str]) -> NormalizedForm {
    form_kind(id, nodes, fps, idents, FormKind::Production)
}

fn form_kind(id: u64, nodes: u32, fps: &[u64], idents: &[&str], kind: FormKind) -> NormalizedForm {
    NormalizedForm {
        id,
        name: format!("f{id}"),
        path: PathBuf::from("a.rs"),
        span: FormSpan::new(1, 10),
        kind,
        node_count: nodes,
        fingerprints: bag(fps),
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
fn exact_cluster_splits_type1_from_renamed_singleton() {
    let forms = [
        form(1, 10, &[1, 2, 3], &["x"]),
        form(2, 10, &[1, 2, 3], &["x"]),
        form(3, 10, &[1, 2, 3], &["y"]),
    ];
    let findings = compare(&forms, 0.85);
    assert_eq!(findings.len(), 2);
    let type1 = findings.iter().find(|f| f.clone_type == crate::domain::CloneType::Type1);
    assert!(type1.is_some_and(|f| f.members.len() == 2));
    let type2 = findings.iter().find(|f| f.clone_type == crate::domain::CloneType::Type2);
    assert!(type2.is_some_and(|f| f.members.len() == 3));
}

#[test]
fn exact_cluster_emits_type1_and_type2_leftovers() {
    let forms = [
        form(1, 10, &[1, 2, 3], &["a"]),
        form(2, 10, &[1, 2, 3], &["a"]),
        form(3, 10, &[1, 2, 3], &["b"]),
        form(4, 10, &[1, 2, 3], &["c"]),
    ];
    let findings = compare(&forms, 0.85);
    assert_eq!(findings.len(), 2);
    assert!(findings.iter().any(|f| f.clone_type == crate::domain::CloneType::Type1));
    assert!(findings.iter().any(|f| f.clone_type == crate::domain::CloneType::Type2));
    let type1 = findings.iter().find(|f| f.clone_type == crate::domain::CloneType::Type1);
    assert!(type1.is_some_and(|f| f.members.len() == 2));
    let type2 = findings.iter().find(|f| f.clone_type == crate::domain::CloneType::Type2);
    assert!(type2.is_some_and(|f| f.members.len() == 4));
}

#[test]
fn distinct_bags_get_distinct_bucket_keys() {
    let left = form(1, 10, &[1, 2], &["x"]);
    let right = form(2, 10, &[3], &["y"]);
    assert_ne!(left.bucket_key(), right.bucket_key());
    assert_ne!(left.fingerprints, right.fingerprints);
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
    // dry-rs:ignore. Shares empty-result shape with disjoint-fingerprint case.
    let left = form(1, 4, &[1, 2, 3, 4], &["a"]);
    let right = form(2, 4, &[9, 8, 7, 6], &["a"]);
    assert!(compare(&[left, right], 0.9).is_empty());
}

#[test]
fn zero_threshold_keeps_window_open() {
    assert!(within_jaccard_window(1, 100, 0.0));
}

#[test]
fn jaccard_window_orders_sizes() {
    assert!(!within_jaccard_window(2, 20, 0.9));
    assert!(!within_jaccard_window(20, 2, 0.9));
    assert!(within_jaccard_window(5, 5, 0.9));
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
fn differing_multiplicity_is_not_exact_match() {
    let left = NormalizedForm {
        id: 1,
        name: "f1".to_owned(),
        path: PathBuf::from("a.rs"),
        span: FormSpan::new(1, 10),
        kind: FormKind::Production,
        node_count: 10,
        fingerprints: BTreeMap::from([(1, 2), (2, 1)]),
        ident_trace: vec!["x".to_owned()],
    };
    let right = NormalizedForm {
        id: 2,
        name: "f2".to_owned(),
        path: PathBuf::from("a.rs"),
        span: FormSpan::new(1, 10),
        kind: FormKind::Production,
        node_count: 10,
        fingerprints: BTreeMap::from([(1, 1), (2, 1)]),
        ident_trace: vec!["y".to_owned()],
    };
    let findings = compare(&[left, right], 0.5);
    assert!(findings.iter().all(|f| f.score < 1.0));
    assert!(!findings.is_empty());
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

fn assert_type3_group(
    forms: &[NormalizedForm],
    threshold: f64,
    member_count: usize,
    expected_score: f64,
) {
    let findings = compare(forms, threshold);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].members.len(), member_count);
    assert_eq!(findings[0].clone_type, crate::domain::CloneType::Type3);
    assert!((findings[0].score - expected_score).abs() < f64::EPSILON);
}

#[test]
fn near_miss_clusters_clique_and_splits_chain() {
    // Clique: all pairs near-miss → one 3-member finding.
    // Chain: A~B and B~C only (A~C below threshold) → exclusive pair.
    assert_type3_group(
        &[
            form(1, 4, &[1, 2, 3, 4], &["a"]),
            form(2, 4, &[1, 2, 3, 5], &["a"]),
            form(3, 4, &[1, 2, 3, 6], &["a"]),
        ],
        0.5,
        3,
        0.6,
    );
    assert_type3_group(
        &[
            form(1, 4, &[1, 2, 3, 4], &["a"]),
            form(2, 4, &[1, 2, 3, 5], &["a"]),
            form(3, 4, &[2, 3, 5, 6], &["a"]),
        ],
        0.5,
        2,
        0.6,
    );
}

#[test]
fn near_miss_chain_does_not_inflate_auto_refactor_tier() {
    // Neighbor edges ≥ 0.95, but A–C is weaker; score must be min pairwise.
    let a_fps: Vec<u64> = (0..40).collect();
    let mut b_fps: Vec<u64> = (1..40).collect();
    b_fps.push(40);
    let mut c_fps: Vec<u64> = (2..40).collect();
    c_fps.extend([40, 41]);
    let forms = [
        form(1, 40, &a_fps, &["a"]),
        form(2, 40, &b_fps, &["a"]),
        form(3, 40, &c_fps, &["a"]),
    ];
    let findings = compare(&forms, 0.85);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].members.len(), 3);
    assert!(findings[0].score < AUTO_REFACTOR_FLOOR);
    assert!(findings[0].score >= REVIEW_FIRST_FLOOR);
    assert_eq!(findings[0].tier, crate::domain::Tier::ReviewFirst);
    assert!(findings[0].score >= 0.85);
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
    // dry-rs:ignore. Shares empty-result shape with below-threshold case.
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

#[test]
fn collect_near_miss_edges_skips_missing_index_postings() {
    let left = form(1, 5, &[1, 2, 3, 4, 99], &["a"]);
    let right = form(2, 5, &[1, 2, 3, 4, 9], &["a"]);
    let remaining = vec![&left, &right];
    let edges = collect_near_miss_edges(&remaining, 0.5);
    assert_eq!(edges.len(), 1);
}

fn form_counts(id: u64, nodes: u32, pairs: &[(u64, u32)], idents: &[&str]) -> NormalizedForm {
    NormalizedForm {
        id,
        name: format!("f{id}"),
        path: PathBuf::from("a.rs"),
        span: FormSpan::new(1, 10),
        kind: FormKind::Production,
        node_count: nodes,
        fingerprints: pairs.iter().copied().collect(),
        ident_trace: idents.iter().map(|s| (*s).to_owned()).collect(),
    }
}

#[test]
fn near_miss_recall_when_overlap_is_mostly_common_leaf() {
    // Multiset Jaccard ≈ 0.94 from a shared high-DF leaf plus small shared tail.
    let forms = [
        form_counts(1, 100, &[(1, 94), (10, 3), (11, 3)], &["a"]),
        form_counts(2, 100, &[(1, 94), (10, 3), (12, 3)], &["a"]),
        // Raises DF of leaf 1 without pairing against the near-miss twins.
        form_counts(3, 100, &[(1, 10), (30, 45), (31, 45)], &["a"]),
    ];
    let findings = compare(&forms, 0.85);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].members.len(), 2);
    assert!(findings[0].score >= 0.85);
    assert!(findings[0].score < 1.0);
}

#[test]
fn prefix_filter_avoids_common_leaf_all_pairs_clique() {
    // Many forms share only leaf 1; each has a unique rare key. Prefix mass is 1
    // at threshold 0.85 for bag_size 2, so only the rare key is indexed/probed.
    let mut forms = Vec::new();
    for i in 0..40_u64 {
        forms.push(form(i + 1, 2, &[1, 1000 + i], &["a"]));
    }
    let remaining: Vec<&NormalizedForm> = forms.iter().collect();
    let df = fingerprint_df_for_test(&remaining);
    for form in &forms {
        let keys = prefix_keys(form, &df, 0.85);
        let rare = form.fingerprints.keys().copied().filter(|&fp| fp != 1).collect::<Vec<_>>();
        assert_eq!(rare.len(), 1);
        assert_eq!(keys, rare);
        assert!(!keys.contains(&1));
    }
    let edges = collect_near_miss_edges(&remaining, 0.85);
    assert!(edges.is_empty());
    assert!(compare(&forms, 0.85).is_empty());
}
