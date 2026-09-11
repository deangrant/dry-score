//! Extract functions and methods from a `syn::File`.

use std::path::Path;

use syn::visit::Visit;
use syn::{Attribute, File, ImplItem, Item, ItemFn, ItemImpl, ItemTrait, TraitItem};

use dry_core::{FingerprintResult, NormalizedForm, below_size_thresholds, fingerprint_tree};

use super::FormParts;
use super::build_form;
use super::emit::emit_block;
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
        let is_test = self.in_test_cfg || has_test_attr(&node.attrs) || has_cfg_test(&node.attrs);
        self.maybe_emit_fn(&node.sig.ident.to_string(), &node.block, &node.sig, is_test);
        syn::visit::visit_item_fn(self, node);
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
        }
        syn::visit::visit_item_impl(self, node);
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
        }
        syn::visit::visit_item_trait(self, node);
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
        let tree = emit_block(block, &mut placeholders);
        let fp = fingerprint_tree(&tree);
        if below_size_thresholds(fp.node_count, start, end, self.min_nodes, self.min_lines) {
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

    #[test]
    fn extracts_trait_default_methods_only() {
        let source = r"
            trait T {
                type Item;
                const N: i32;
                fn required(&self);
                fn with_default(&self, x: i32) {
                    let y = x + 1;
                    let z = y + 2;
                    let w = z + 3;
                }
                #[test]
                fn tested_default(&self) {
                    let a = 1;
                    let b = a + 1;
                    let c = b + 1;
                }
            }
        ";
        let parsed = parse_file(source);
        assert!(parsed.is_ok());
        #[expect(clippy::expect_used, reason = "test asserts parse ok")]
        let file = parsed.expect("parse");
        let mut next_id = 1;
        let forms = extract_forms(&file, Path::new("t.rs"), source, 5, 3, &mut next_id);
        assert!(forms.iter().any(|f| f.name == "T::with_default"));
        assert!(forms.iter().any(|f| f.name == "T::tested_default"));
        assert!(
            forms
                .iter()
                .any(|f| f.name == "T::tested_default" && f.kind == dry_core::FormKind::Test)
        );
        assert!(!forms.iter().any(|f| f.name == "T::required"));
    }

    fn extract(source: &str) -> Vec<NormalizedForm> {
        let parsed = parse_file(source);
        assert!(parsed.is_ok());
        #[expect(clippy::expect_used, reason = "test asserts parse ok")]
        let file = parsed.expect("parse");
        let mut next_id = 1;
        extract_forms(&file, Path::new("t.rs"), source, 5, 3, &mut next_id)
    }

    fn form_kind(forms: &[NormalizedForm], name: &str) -> Option<dry_core::FormKind> {
        forms.iter().find(|f| f.name == name).map(|f| f.kind)
    }

    #[test]
    fn classifies_tokio_test_and_cfg_test_fn() {
        let source = r"
            fn production_twin() {
                let a = 1;
                let b = a + 1;
                let c = b + 1;
            }
            #[tokio::test]
            async fn tokio_case() {
                let a = 1;
                let b = a + 1;
                let c = b + 1;
            }
            #[cfg(test)]
            fn cfg_helper() {
                let a = 1;
                let b = a + 1;
                let c = b + 1;
            }
        ";
        let forms = extract(source);
        assert_eq!(
            form_kind(&forms, "production_twin"),
            Some(dry_core::FormKind::Production)
        );
        assert_eq!(
            form_kind(&forms, "tokio_case"),
            Some(dry_core::FormKind::Test)
        );
        assert_eq!(
            form_kind(&forms, "cfg_helper"),
            Some(dry_core::FormKind::Test)
        );
    }

    #[test]
    fn classifies_compound_cfg_modules_and_not_test() {
        let source = r#"
            #[cfg(any(test, feature = "x"))]
            mod any_tests {
                fn inside_any() {
                    let a = 1;
                    let b = a + 1;
                    let c = b + 1;
                }
            }
            #[cfg(not(test))]
            fn not_test_helper() {
                let a = 1;
                let b = a + 1;
                let c = b + 1;
            }
        "#;
        let forms = extract(source);
        assert_eq!(
            form_kind(&forms, "inside_any"),
            Some(dry_core::FormKind::Test)
        );
        assert_eq!(
            form_kind(&forms, "not_test_helper"),
            Some(dry_core::FormKind::Production)
        );

        let any_cfg: syn::Attribute = syn::parse_quote!(#[cfg(any(test, feature = "x"))]);
        assert!(attr_is_cfg_test(&any_cfg));
        let all_cfg: syn::Attribute = syn::parse_quote!(#[cfg(all(test, unix))]);
        assert!(attr_is_cfg_test(&all_cfg));
        let not_cfg: syn::Attribute = syn::parse_quote!(#[cfg(not(test))]);
        assert!(!attr_is_cfg_test(&not_cfg));
        let feature_named_test: syn::Attribute = syn::parse_quote!(#[cfg(feature = "test")]);
        assert!(!attr_is_cfg_test(&feature_named_test));
    }

    #[test]
    fn unused_params_do_not_shift_body_placeholders() {
        let source = r"
            fn with_unused(unused: i32, x: i32) {
                let y = x + 1;
                let z = y + 2;
                let w = z + 3;
            }
            fn without_unused(x: i32) {
                let y = x + 1;
                let z = y + 2;
                let w = z + 3;
            }
        ";
        let forms = extract(source);
        let left = forms.iter().find(|f| f.name == "with_unused");
        let right = forms.iter().find(|f| f.name == "without_unused");
        assert!(left.is_some());
        assert!(right.is_some());
        #[expect(clippy::expect_used, reason = "test asserts forms exist")]
        let left = left.expect("with_unused");
        #[expect(clippy::expect_used, reason = "test asserts forms exist")]
        let right = right.expect("without_unused");
        assert_eq!(left.fingerprints, right.fingerprints);
    }
}
