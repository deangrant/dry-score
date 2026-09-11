//! Extract functions and methods from a `syn::File`.

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Attribute, Expr, File, ImplItem, Item, ItemFn, ItemImpl, ItemTrait, TraitItem};

use dry_core::{FingerprintResult, NormalizedForm, below_size_thresholds, fingerprint_tree};

use super::FormParts;
use super::build_form;
use super::emit::{emit_block, emit_expr};
use super::kind_from_test;
use super::placeholders::PlaceholderMap;
use super::suppress::span_is_ignored;

/// Walks a parsed file and emits size-filtered forms.
pub fn extract_forms(
    file: &File,
    path: &Path,
    source: &str,
    min_nodes: u32,
    min_lines: u32,
    next_id: &mut u64,
) -> Vec<NormalizedForm> {
    let mut extractor = Extractor {
        path,
        source,
        min_nodes,
        min_lines,
        next_id,
        in_test_cfg: false,
        enclosing_name: None,
        enclosing_is_test: false,
        forms: Vec::new(),
    };
    extractor.visit_file(file);
    extractor.forms
}

struct Extractor<'a> {
    path: &'a Path,
    source: &'a str,
    min_nodes: u32,
    min_lines: u32,
    next_id: &'a mut u64,
    in_test_cfg: bool,
    enclosing_name: Option<String>,
    enclosing_is_test: bool,
    forms: Vec<NormalizedForm>,
}

impl<'ast> Visit<'ast> for Extractor<'_> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let is_test = self.in_test_cfg || has_test_attr(&node.attrs) || has_cfg_test(&node.attrs);
        let name = node.sig.ident.to_string();
        self.maybe_emit_fn(&name, &node.block, &node.sig, is_test);
        let previous_name = self.enclosing_name.replace(name);
        let previous_test = self.enclosing_is_test;
        self.enclosing_is_test = is_test;
        syn::visit::visit_item_fn(self, node);
        self.enclosing_name = previous_name;
        self.enclosing_is_test = previous_test;
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let impl_is_test = self.in_test_cfg || has_cfg_test(&node.attrs);
        let impl_name = impl_type_name(node);
        for item in &node.items {
            let ImplItem::Fn(method) = item else {
                continue;
            };
            let name = format!("{impl_name}::{}", method.sig.ident);
            let is_test =
                impl_is_test || has_test_attr(&method.attrs) || has_cfg_test(&method.attrs);
            self.maybe_emit_fn(&name, &method.block, &method.sig, is_test);
            let previous_name = self.enclosing_name.replace(name);
            let previous_test = self.enclosing_is_test;
            self.enclosing_is_test = is_test;
            syn::visit::visit_impl_item_fn(self, method);
            self.enclosing_name = previous_name;
            self.enclosing_is_test = previous_test;
        }
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        let trait_is_test = self.in_test_cfg || has_cfg_test(&node.attrs);
        let trait_name = node.ident.to_string();
        for item in &node.items {
            let TraitItem::Fn(method) = item else {
                continue;
            };
            let Some(block) = &method.default else {
                continue;
            };
            let name = format!("{trait_name}::{}", method.sig.ident);
            let is_test =
                trait_is_test || has_test_attr(&method.attrs) || has_cfg_test(&method.attrs);
            self.maybe_emit_fn(&name, block, &method.sig, is_test);
            let previous_name = self.enclosing_name.replace(name);
            let previous_test = self.enclosing_is_test;
            self.enclosing_is_test = is_test;
            syn::visit::visit_trait_item_fn(self, method);
            self.enclosing_name = previous_name;
            self.enclosing_is_test = previous_test;
        }
    }

    fn visit_item(&mut self, node: &'ast Item) {
        let entered_test = matches!(node, Item::Mod(module) if has_cfg_test(&module.attrs));
        let previous = self.in_test_cfg;
        if entered_test {
            self.in_test_cfg = true;
        }
        syn::visit::visit_item(self, node);
        self.in_test_cfg = previous;
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.maybe_emit_closure(node);
        syn::visit::visit_expr_closure(self, node);
    }
}

impl Extractor<'_> {
    fn maybe_emit_fn(
        &mut self,
        name: &str,
        block: &syn::Block,
        sig: &syn::Signature,
        is_test: bool,
    ) {
        let start = u32::try_from(sig.fn_token.span.start().line).unwrap_or(1);
        let end = u32::try_from(block.brace_token.span.close().end().line).unwrap_or(start);
        if span_is_ignored(self.source, start, end) {
            return;
        }
        let mut placeholders = PlaceholderMap::default();
        let tree = emit_block(block, &mut placeholders);
        let fp = fingerprint_tree(&tree);
        if below_size_thresholds(fp.node_count, start, end, self.min_nodes, self.min_lines) {
            return;
        }
        self.push_form(name, start, end, is_test, fp, placeholders.ident_trace);
    }

    fn maybe_emit_closure(&mut self, closure: &syn::ExprClosure) {
        let start = u32::try_from(closure.span().start().line).unwrap_or(1);
        let end = u32::try_from(closure.span().end().line).unwrap_or(start);
        if span_is_ignored(self.source, start, end) {
            return;
        }
        let mut placeholders = PlaceholderMap::default();
        let tree = match &*closure.body {
            Expr::Block(block) => emit_block(&block.block, &mut placeholders),
            other => emit_expr(other, &mut placeholders),
        };
        let fp = fingerprint_tree(&tree);
        if below_size_thresholds(fp.node_count, start, end, self.min_nodes, self.min_lines) {
            return;
        }
        let name = self.enclosing_name.as_ref().map_or_else(
            || format!("$closure:L{start}"),
            |parent| format!("{parent}.$closure:L{start}"),
        );
        self.push_form(
            &name,
            start,
            end,
            self.enclosing_is_test || self.in_test_cfg,
            fp,
            placeholders.ident_trace,
        );
    }

    fn push_form(
        &mut self,
        name: &str,
        start: u32,
        end: u32,
        is_test: bool,
        fp: FingerprintResult,
        ident_trace: Vec<String>,
    ) {
        let id = *self.next_id;
        *self.next_id = self.next_id.saturating_add(1);
        let kind = kind_from_test(is_test);
        self.forms.push(build_form(FormParts {
            id,
            name: name.to_owned(),
            path: self.path.to_path_buf(),
            start_line: start,
            end_line: end,
            kind,
            node_count: fp.node_count,
            fingerprints: fp.fingerprints,
            ident_trace,
        }));
    }
}

fn has_test_attr(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path().segments.last().is_some_and(|seg| seg.ident == "test"))
}

fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(attr_is_cfg_test)
}

fn attr_is_cfg_test(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let Ok(meta) = attr.parse_args::<syn::Meta>() else {
        return false;
    };
    meta_has_positive_test(&meta, true)
}

/// True when `test` appears under positive polarity in a cfg predicate tree.
fn meta_has_positive_test(meta: &syn::Meta, positive: bool) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test") && positive,
        syn::Meta::List(list) if list.path.is_ident("not") => meta_list_any(list, !positive),
        syn::Meta::List(list) if list.path.is_ident("any") || list.path.is_ident("all") => {
            meta_list_any(list, positive)
        }
        syn::Meta::List(_) | syn::Meta::NameValue(_) => false,
    }
}

fn meta_list_any(list: &syn::MetaList, positive: bool) -> bool {
    let Ok(items) = list.parse_args_with(
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
    ) else {
        return false;
    };
    items.iter().any(|child| meta_has_positive_test(child, positive))
}

fn impl_type_name(node: &ItemImpl) -> String {
    match &*node.self_ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map_or_else(|| "Impl".to_owned(), |seg| seg.ident.to_string()),
        _ => "Impl".to_owned(),
    }
}

#[cfg(test)]
mod tests;
