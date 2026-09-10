//! Shared literal labels for expr, pattern, and macro emission.

use syn::Lit;

/// Stable kind label for a Rust literal.
#[must_use]
pub const fn lit_label(lit: &Lit) -> &'static str {
    match lit_text_label(lit) {
        Some(label) => label,
        None => lit_numeric_label(lit),
    }
}

const fn lit_text_label(lit: &Lit) -> Option<&'static str> {
    match lit {
        Lit::Str(_) => Some("lit_str"),
        Lit::ByteStr(_) => Some("lit_bytestr"),
        Lit::CStr(_) => Some("lit_cstr"),
        _ => lit_byte_char_label(lit),
    }
}

const fn lit_byte_char_label(lit: &Lit) -> Option<&'static str> {
    match lit {
        Lit::Byte(_) => Some("lit_byte"),
        Lit::Char(_) => Some("lit_char"),
        _ => None,
    }
}

const fn lit_numeric_label(lit: &Lit) -> &'static str {
    match lit {
        Lit::Int(_) => "lit_int",
        Lit::Float(_) => "lit_float",
        Lit::Bool(_) => "lit_bool",
        _ => "lit_other",
    }
}

#[cfg(test)]
mod tests {
    use super::lit_label;
    use syn::parse_quote;

    #[test]
    #[expect(
        clippy::cognitive_complexity,
        reason = "literal label corpus is intentionally flat assertions"
    )]
    fn literal_labels() {
        assert_eq!(lit_label(&parse_quote!("hi")), "lit_str");
        assert_eq!(lit_label(&parse_quote!(b"hi")), "lit_bytestr");
        assert_eq!(lit_label(&parse_quote!(b'x')), "lit_byte");
        assert_eq!(lit_label(&parse_quote!('x')), "lit_char");
        assert_eq!(lit_label(&parse_quote!(1)), "lit_int");
        assert_eq!(lit_label(&parse_quote!(1.5)), "lit_float");
        assert_eq!(lit_label(&parse_quote!(true)), "lit_bool");
        assert_eq!(lit_label(&parse_quote!(c"hi")), "lit_cstr");
        assert_eq!(
            lit_label(&syn::Lit::Verbatim(proc_macro2::Literal::i32_unsuffixed(0))),
            "lit_other"
        );
    }
}
