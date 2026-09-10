//! Extract functions and methods from a `syn::File`.

use std::path::Path;

use syn::visit::Visit;
use syn::{Attribute, File, ImplItem, Item, ItemFn, ItemImpl};

use dry_core::NormalizedForm;

use super::FormParts;
use super::build_form;
use super::emit::emit_block;
use super::fingerprint::fingerprint_tree;
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
    forms: Vec<NormalizedForm>,
}

impl<'ast> Visit<'ast> for Extractor<'_> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let is_test = self.in_test_cfg || has_test_attr(&node.attrs);
        self.maybe_emit_fn(&node.sig.ident.to_string(), &node.block, &node.sig, is_test);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let impl_name = impl_type_name(node);
        for item in &node.items {
            let ImplItem::Fn(method) = item else {
                continue;
            };
            let name = format!("{impl_name}::{}", method.sig.ident);
            let is_test = self.in_test_cfg || has_test_attr(&method.attrs);
            self.maybe_emit_fn(&name, &method.block, &method.sig, is_test);
        }
        syn::visit::visit_item_impl(self, node);
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
        bind_signature(sig, &mut placeholders);
        let tree = emit_block(block, &mut placeholders);
        let fp = fingerprint_tree(&tree);
        if below_thresholds(fp.node_count, start, end, self.min_nodes, self.min_lines) {
            return;
        }
        self.push_form(name, start, end, is_test, fp, placeholders.ident_trace);
    }

    fn push_form(
        &mut self,
        name: &str,
        start: u32,
        end: u32,
        is_test: bool,
        fp: super::fingerprint::FingerprintResult,
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

const fn below_thresholds(
    node_count: u32,
    start: u32,
    end: u32,
    min_nodes: u32,
    min_lines: u32,
) -> bool {
    let line_count = end.saturating_sub(start).saturating_add(1);
    node_count < min_nodes || line_count < min_lines
}

fn bind_signature(sig: &syn::Signature, placeholders: &mut PlaceholderMap) {
    for input in &sig.inputs {
        bind_sig_input(input, placeholders);
    }
}

fn bind_sig_input(input: &syn::FnArg, placeholders: &mut PlaceholderMap) {
    match input {
        syn::FnArg::Receiver(_) => {
            let _ = placeholders.placeholder("self");
        }
        syn::FnArg::Typed(pat_type) => {
            if let syn::Pat::Ident(ident) = &*pat_type.pat {
                let _ = placeholders.placeholder(&ident.ident.to_string());
            }
        }
    }
}

fn has_test_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("test"))
}

fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(attr_is_cfg_test)
}

fn attr_is_cfg_test(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let mut is_test = false;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("test") {
            is_test = true;
        }
        Ok(())
    });
    is_test
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
mod tests {
    use super::*;
    use syn::parse_file;

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "extract corpus covers impl/cfg/ignore in one fixture"
    )]
    fn extracts_impl_methods_and_cfg_test() {
        let source = r"
            struct S;
            impl S {
                fn method(&self, x: i32) {
                    let y = x + 1;
                    let z = y + 2;
                    let w = z + 3;
                }
                const N: i32 = 1;
            }
            impl dyn Send {
                fn odd(self: Box<Self>) {
                    let a = 1;
                    let b = a + 1;
                    let c = b + 1;
                }
            }
            #[cfg(test)]
            mod tests {
                #[test]
                fn t() {
                    let a = 1;
                    let b = a + 1;
                    let c = b + 1;
                }
            }
            fn ignored() {
                // dry-rs:ignore
                let a = 1;
                let b = a + 1;
                let c = b + 1;
            }
            fn tiny() { 1 }
            #[allow(dead_code)]
            fn with_other_attr() {
                let a = 1;
                let b = a + 1;
                let c = b + 1;
            }
        ";
        let parsed = parse_file(source);
        assert!(parsed.is_ok());
        #[expect(clippy::expect_used, reason = "test asserts parse ok")]
        let file = parsed.expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(&file, Path::new("t.rs"), source, 5, 3, &mut next_id);
        assert!(forms.iter().any(|f| f.name.contains("method")));
        assert!(forms.iter().any(|f| f.name.starts_with("Impl::")));
        assert!(forms.iter().any(|f| f.kind == dry_core::FormKind::Test));
        assert!(!forms.iter().any(|f| f.name == "ignored"));
        let cfg: syn::Attribute = syn::parse_quote!(#[cfg(test)]);
        assert!(attr_is_cfg_test(&cfg));
        let other: syn::Attribute = syn::parse_quote!(#[allow(dead_code)]);
        assert!(!attr_is_cfg_test(&other));
    }
}
