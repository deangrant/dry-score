//! Extract Python functions, methods, and lambdas from a CST.

mod forms;
mod names;

use std::path::Path;

use dry_core::{
    FormKind, FormSpan, NormalizedForm, PlaceholderMap, below_size_thresholds, fingerprint_tree,
};
use tree_sitter::Node;

use super::emit::{emit_node, should_skip};
use super::kind_from_path;
use super::suppress::span_is_ignored;
use forms::{try_walk_decorated, try_walk_function, try_walk_lambda};

/// Walks a parsed file and emits size-filtered forms.
pub(super) fn extract_forms(
    root: Node<'_>,
    path: &Path,
    source_bytes: &[u8],
    source: &str,
    min_nodes: u32,
    min_lines: u32,
    next_id: &mut u64,
) -> Vec<NormalizedForm> {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    let mut ctx = ExtractCtx {
        path,
        source_bytes,
        source,
        min_nodes,
        min_lines,
        next_id,
        kind: kind_from_path(path),
        forms: Vec::new(),
    };
    walk_forms(root, None, None, &mut ctx);
    ctx.forms
}

pub(super) struct ExtractCtx<'a> {
    pub path: &'a Path,
    pub source_bytes: &'a [u8],
    pub source: &'a str,
    pub min_nodes: u32,
    pub min_lines: u32,
    pub next_id: &'a mut u64,
    pub kind: FormKind,
    pub forms: Vec<NormalizedForm>,
}

pub(super) fn walk_forms(
    node: Node<'_>,
    parent_name: Option<&str>,
    class_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) {
    // dry-rs:ignore. CC-driven CST kind dispatch; parallel shape is intentional.
    if try_walk_function(node, class_name, ctx) {
        return;
    }
    if try_walk_lambda(node, parent_name, ctx) {
        return;
    }
    if try_walk_decorated(node, class_name, ctx) {
        return;
    }
    walk_children(node, parent_name, class_name, ctx);
}

/// Walks named children, propagating binding / class context from `node`.
pub(super) fn walk_children(
    node: Node<'_>,
    parent_name: Option<&str>,
    class_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    let next_class =
        class_name_from(node, ctx.source_bytes).or_else(|| class_name.map(str::to_owned));
    let next_parent =
        binding_name_from(node, ctx.source_bytes).or_else(|| parent_name.map(str::to_owned));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if should_skip(child) {
            continue;
        }
        walk_forms(child, next_parent.as_deref(), next_class.as_deref(), ctx);
    }
}

fn class_name_from(node: Node<'_>, source: &[u8]) -> Option<String> {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    if node.kind() != "class_definition" {
        return None;
    }
    node.child_by_field_name("name")
        .map(|n| super::emit::node_text(n, source).to_owned())
}

fn binding_name_from(node: Node<'_>, source: &[u8]) -> Option<String> {
    let field = binding_field(node.kind())?;
    let left = node.child_by_field_name(field)?;
    binding_ident_text(left, source)
}

fn binding_field(kind: &str) -> Option<&'static str> {
    match kind {
        "assignment" => Some("left"),
        "pair" => Some("key"),
        _ => None,
    }
}

fn binding_ident_text(left: Node<'_>, source: &[u8]) -> Option<String> {
    if left.kind() == "identifier" || left.kind() == "string" {
        Some(super::emit::node_text(left, source).to_owned())
    } else {
        None
    }
}

pub(super) fn push_form(body: Node<'_>, name: &str, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    let start = u32::try_from(body.start_position().row.saturating_add(1)).unwrap_or(1);
    let end = u32::try_from(body.end_position().row.saturating_add(1)).unwrap_or(start);
    if span_is_ignored(ctx.source, start, end) {
        return;
    }
    let mut placeholders = PlaceholderMap::default();
    let tree = emit_node(body, ctx.source_bytes, &mut placeholders);
    let fp = fingerprint_tree(&tree);
    if below_size_thresholds(fp.node_count, start, end, ctx.min_nodes, ctx.min_lines) {
        return;
    }
    let id = *ctx.next_id;
    *ctx.next_id = ctx.next_id.saturating_add(1);
    ctx.forms.push(NormalizedForm {
        id,
        name: name.to_owned(),
        path: ctx.path.to_path_buf(),
        span: FormSpan {
            start_line: start,
            end_line: end,
        },
        kind: ctx.kind,
        node_count: fp.node_count,
        fingerprints: fp.fingerprints,
        ident_trace: placeholders.ident_trace,
    });
}

#[cfg(test)]
mod tests;
