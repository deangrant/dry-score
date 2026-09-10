//! Pure structural helpers shared by expr and pattern emitters (no emit recursion).

use syn::{Member, Path};

use crate::normalize::placeholders::PlaceholderMap;
use crate::normalize::tree::NormNode;

/// Named or unnamed struct/tuple field spelling for placeholders.
pub(super) fn member_name(member: &Member) -> String {
    match member {
        Member::Named(ident) => ident.to_string(),
        Member::Unnamed(index) => index.index.to_string(),
    }
}

/// Emits a path as a single-ident placeholder or multi-segment `path:` leaf.
pub(super) fn emit_path_segments(path: &Path, placeholders: &mut PlaceholderMap) -> NormNode {
    if let Some(ident) = path.get_ident() {
        return NormNode::leaf(placeholders.placeholder(&ident.to_string()));
    }
    let label = path
        .segments
        .iter()
        .map(|seg| placeholders.placeholder(&seg.ident.to_string()))
        .collect::<Vec<_>>()
        .join("::");
    NormNode::leaf(format!("path:{label}"))
}

/// Placeholder leaves for each path segment (struct/tuple-struct path heads).
pub(super) fn path_segment_leaves(path: &Path, placeholders: &mut PlaceholderMap) -> Vec<NormNode> {
    path.segments
        .iter()
        .map(|seg| NormNode::leaf(placeholders.placeholder(&seg.ident.to_string())))
        .collect()
}
