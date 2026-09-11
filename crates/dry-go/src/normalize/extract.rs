//! Extract Go functions, methods, and func literals from a CST.

use std::path::Path;

use dry_core::{
    FormKind, FormSpan, NormalizedForm, PlaceholderMap, below_size_thresholds, fingerprint_tree,
};
use tree_sitter::Node;

use super::emit::{emit_node, node_text};
use super::kind_from_path;
use super::suppress::span_is_ignored;

/// Walks a parsed file and emits size-filtered forms.
pub fn extract_forms(
    root: Node<'_>,
    path: &Path,
    source_bytes: &[u8],
    source: &str,
    min_nodes: u32,
    min_lines: u32,
    next_id: &mut u64,
) -> Vec<NormalizedForm> {
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
    walk_forms(root, None, &mut ctx);
    ctx.forms
}

struct ExtractCtx<'a> {
    path: &'a Path,
    source_bytes: &'a [u8],
    source: &'a str,
    min_nodes: u32,
    min_lines: u32,
    next_id: &'a mut u64,
    kind: FormKind,
    forms: Vec<NormalizedForm>,
}

fn walk_forms(node: Node<'_>, parent_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    match node.kind() {
        "function_declaration" => {
            maybe_emit_named(node, ctx);
            let name = function_name(node, ctx.source_bytes);
            walk_children(node, name.as_deref(), ctx);
        }
        "method_declaration" => {
            maybe_emit_named(node, ctx);
            let name = method_name(node, ctx.source_bytes);
            walk_children(node, name.as_deref(), ctx);
        }
        "func_literal" => {
            maybe_emit_literal(node, parent_name, ctx);
            walk_children(node, parent_name, ctx);
        }
        _ => walk_children(node, parent_name, ctx),
    }
}

fn walk_children(node: Node<'_>, parent_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() || child.kind() == "comment" {
            continue;
        }
        walk_forms(child, parent_name, ctx);
    }
}

fn maybe_emit_named(node: Node<'_>, ctx: &mut ExtractCtx<'_>) {
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let name = match node.kind() {
        "method_declaration" => method_name(node, ctx.source_bytes),
        _ => function_name(node, ctx.source_bytes),
    };
    let Some(name) = name else {
        return;
    };
    push_form(body, &name, ctx);
}

fn maybe_emit_literal(node: Node<'_>, parent_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let line = u32::try_from(node.start_position().row.saturating_add(1)).unwrap_or(u32::MAX);
    let name = parent_name.map_or_else(
        || format!("$literal:L{line}"),
        |parent| format!("{parent}.$literal:L{line}"),
    );
    push_form(body, &name, ctx);
}

fn push_form(body: Node<'_>, name: &str, ctx: &mut ExtractCtx<'_>) {
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

fn function_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name").map(|n| node_text(n, source).to_owned())
}

fn method_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let method = node.child_by_field_name("name").map(|n| node_text(n, source))?;
    let recv = receiver_type_name(node, source).unwrap_or_else(|| "_".to_owned());
    Some(format!("{recv}::{method}"))
}

fn receiver_type_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let params = node.child_by_field_name("receiver")?;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() != "parameter_declaration" {
            continue;
        }
        if let Some(ty) = child.child_by_field_name("type") {
            return Some(strip_pointer(ty, source));
        }
    }
    None
}

fn strip_pointer(node: Node<'_>, source: &[u8]) -> String {
    if node.kind() == "pointer_type"
        && let Some(inner) = node.named_child(0)
    {
        return strip_pointer(inner, source);
    }
    if node.kind() == "type_identifier" || node.kind() == "identifier" {
        return node_text(node, source).to_owned();
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            return strip_pointer(child, source);
        }
    }
    node_text(node, source).to_owned()
}
