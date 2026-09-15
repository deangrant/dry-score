//! CST walk helpers that emit Python function-like forms.

use tree_sitter::Node;

use super::names::{lambda_name, method_name};
use super::{ExtractCtx, push_form, walk_children};

pub(super) fn try_walk_function(
    node: Node<'_>,
    class_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) -> bool {
    // dry-rs:ignore. CC-driven CST walk helpers; parallel shape is intentional.
    if node.kind() != "function_definition" {
        return false;
    }
    maybe_emit_function(node, class_name, ctx);
    let name = method_name(node, class_name, ctx.source_bytes);
    walk_children(node, name.as_deref(), class_name, ctx);
    true
}

pub(super) fn try_walk_lambda(
    node: Node<'_>,
    parent_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) -> bool {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    if node.kind() != "lambda" {
        return false;
    }
    maybe_emit_lambda(node, parent_name, ctx);
    walk_children(node, parent_name, None, ctx);
    true
}

pub(super) fn try_walk_decorated(
    node: Node<'_>,
    class_name: Option<&str>,
    ctx: &mut ExtractCtx<'_>,
) -> bool {
    if node.kind() != "decorated_definition" {
        return false;
    }
    // Walk the inner definition with the same class context.
    walk_children(node, None, class_name, ctx);
    true
}

fn maybe_emit_function(node: Node<'_>, class_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. CC-driven emit shells; parallel shape is intentional.
    if let Some(body) = node.child_by_field_name("body")
        && let Some(name) = method_name(node, class_name, ctx.source_bytes)
    {
        push_form(body, &name, ctx);
    }
}

fn maybe_emit_lambda(node: Node<'_>, parent_name: Option<&str>, ctx: &mut ExtractCtx<'_>) {
    // dry-rs:ignore. Tree-sitter adapter parallel with dry-go/dry-ts; intentional.
    if let Some(body) = node.child_by_field_name("body") {
        let name = lambda_name(parent_name, node);
        push_form(body, &name, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::super::names::function_name;
    use crate::normalize::parse::parse_source;

    #[test]
    fn function_name_reads_def_name() {
        let src = "def score(n):\n    return n\n";
        #[expect(clippy::expect_used, reason = "test setup")]
        let tree = parse_source(src).expect("parse");
        let root = tree.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "function_definition" {
                assert_eq!(
                    function_name(child, src.as_bytes()).as_deref(),
                    Some("score")
                );
            }
        }
    }
}
