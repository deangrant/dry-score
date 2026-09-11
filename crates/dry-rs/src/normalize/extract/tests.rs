use super::*;
use std::path::Path;
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

#[test]
fn nested_closures_with_identical_bodies_share_fingerprints() {
    let source = r"
            fn with_closures() -> i32 {
                let left = |n: i32| {
                    let mut acc = n;
                    if acc < 0 {
                        acc = 0 - acc;
                    }
                    acc + 1
                };
                let right = |n: i32| {
                    let mut acc = n;
                    if acc < 0 {
                        acc = 0 - acc;
                    }
                    acc + 1
                };
                left(3) + right(4)
            }
        ";
    let forms = extract(source);
    let closures: Vec<_> = forms.iter().filter(|f| f.name.contains("$closure:")).collect();
    assert!(
        closures.len() >= 2,
        "expected nested closure forms, got {forms:?}"
    );
    assert_eq!(closures[0].fingerprints, closures[1].fingerprints);
    assert!(closures.iter().all(|f| f.name.starts_with("with_closures.$closure:")));
}
