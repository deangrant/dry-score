//! Operator label helpers.

use syn::{BinOp, UnOp};

/// Stable label for a binary operator.
#[must_use]
pub fn bin_op_label(op: &BinOp) -> &'static str {
    bin_op_label_debug(&format!("{op:?}"))
}

/// Stable label for a unary operator.
#[must_use]
pub fn un_op_label(op: &UnOp) -> &'static str {
    un_op_label_debug(&format!("{op:?}"))
}

fn bin_op_label_debug(debug: &str) -> &'static str {
    let debug = debug.strip_prefix("BinOp::").unwrap_or(debug);
    assign_label(debug)
        .or_else(|| arith_label(debug))
        .or_else(|| bit_label(debug))
        .or_else(|| cmp_label(debug))
        .unwrap_or("other")
}

fn un_op_label_debug(debug: &str) -> &'static str {
    let debug = debug.strip_prefix("UnOp::").unwrap_or(debug);
    if debug.starts_with("Deref") {
        return "deref";
    }
    if debug.starts_with("Not") {
        return "not";
    }
    if debug.starts_with("Neg") {
        return "neg";
    }
    "other"
}

fn assign_label(debug: &str) -> Option<&'static str> {
    if debug.starts_with("AddAssign") {
        return Some("add_assign");
    }
    if debug.starts_with("SubAssign") {
        return Some("sub_assign");
    }
    if debug.starts_with("MulAssign") {
        return Some("mul_assign");
    }
    if debug.starts_with("DivAssign") {
        return Some("div_assign");
    }
    assign_rem_bit_label(debug)
}

fn assign_rem_bit_label(debug: &str) -> Option<&'static str> {
    if debug.starts_with("RemAssign") {
        return Some("rem_assign");
    }
    if debug.starts_with("BitXorAssign") {
        return Some("bitxor_assign");
    }
    if debug.starts_with("BitAndAssign") {
        return Some("bitand_assign");
    }
    if debug.starts_with("BitOrAssign") {
        return Some("bitor_assign");
    }
    assign_shift_label(debug)
}

fn assign_shift_label(debug: &str) -> Option<&'static str> {
    if debug.starts_with("ShlAssign") {
        return Some("shl_assign");
    }
    if debug.starts_with("ShrAssign") {
        return Some("shr_assign");
    }
    None
}

fn arith_label(debug: &str) -> Option<&'static str> {
    if debug.starts_with("Add") {
        return Some("add");
    }
    if debug.starts_with("Sub") {
        return Some("sub");
    }
    if debug.starts_with("Mul") {
        return Some("mul");
    }
    if debug.starts_with("Div") {
        return Some("div");
    }
    rem_label(debug)
}

fn rem_label(debug: &str) -> Option<&'static str> {
    if debug.starts_with("Rem") {
        Some("rem")
    } else {
        None
    }
}

fn bit_label(debug: &str) -> Option<&'static str> {
    if debug.starts_with("And") {
        return Some("and");
    }
    if debug.starts_with("Or") {
        return Some("or");
    }
    if debug.starts_with("BitXor") {
        return Some("bitxor");
    }
    if debug.starts_with("BitAnd") {
        return Some("bitand");
    }
    bit_or_shift_label(debug)
}

fn bit_or_shift_label(debug: &str) -> Option<&'static str> {
    if debug.starts_with("BitOr") {
        return Some("bitor");
    }
    if debug.starts_with("Shl") {
        return Some("shl");
    }
    if debug.starts_with("Shr") {
        return Some("shr");
    }
    None
}

fn cmp_label(debug: &str) -> Option<&'static str> {
    if debug.starts_with("Eq") {
        return Some("eq");
    }
    if debug.starts_with("Ne") {
        return Some("ne");
    }
    if debug.starts_with("Le") {
        return Some("le");
    }
    if debug.starts_with("Lt") {
        return Some("lt");
    }
    cmp_gt_label(debug)
}

fn cmp_gt_label(debug: &str) -> Option<&'static str> {
    if debug.starts_with("Ge") {
        return Some("ge");
    }
    if debug.starts_with("Gt") {
        return Some("gt");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn label_bin(expr: syn::Expr) -> &'static str {
        let syn::Expr::Binary(bin) = expr else {
            return "not-binary";
        };
        bin_op_label(&bin.op)
    }

    fn label_un(expr: syn::Expr) -> &'static str {
        let syn::Expr::Unary(unary) = expr else {
            return "not-unary";
        };
        un_op_label(&unary.op)
    }

    #[test]
    fn arith_and_logic_labels() {
        assert_eq!(label_bin(parse_quote!(a + b)), "add");
        assert_eq!(label_bin(parse_quote!(a - b)), "sub");
        assert_eq!(label_bin(parse_quote!(a * b)), "mul");
        assert_eq!(label_bin(parse_quote!(a / b)), "div");
        assert_eq!(label_bin(parse_quote!(a % b)), "rem");
        assert_eq!(label_bin(parse_quote!(a && b)), "and");
        assert_eq!(label_bin(parse_quote!(a || b)), "or");
    }

    #[test]
    #[expect(
        clippy::cognitive_complexity,
        reason = "bit/cmp label corpus is intentionally flat assertions"
    )]
    fn bit_and_cmp_labels() {
        assert_eq!(label_bin(parse_quote!(a ^ b)), "bitxor");
        assert_eq!(label_bin(parse_quote!(a & b)), "bitand");
        assert_eq!(label_bin(parse_quote!(a | b)), "bitor");
        assert_eq!(label_bin(parse_quote!(a << b)), "shl");
        assert_eq!(label_bin(parse_quote!(a >> b)), "shr");
        assert_eq!(label_bin(parse_quote!(a == b)), "eq");
        assert_eq!(label_bin(parse_quote!(a != b)), "ne");
        assert_eq!(label_bin(parse_quote!(a < b)), "lt");
        assert_eq!(label_bin(parse_quote!(a <= b)), "le");
        assert_eq!(label_bin(parse_quote!(a > b)), "gt");
        assert_eq!(label_bin(parse_quote!(a >= b)), "ge");
    }

    #[test]
    fn assign_arith_labels() {
        assert_eq!(label_bin(parse_quote!(a += b)), "add_assign");
        assert_eq!(label_bin(parse_quote!(a -= b)), "sub_assign");
        assert_eq!(label_bin(parse_quote!(a *= b)), "mul_assign");
        assert_eq!(label_bin(parse_quote!(a /= b)), "div_assign");
        assert_eq!(label_bin(parse_quote!(a %= b)), "rem_assign");
    }

    #[test]
    fn assign_bit_labels() {
        assert_eq!(label_bin(parse_quote!(a ^= b)), "bitxor_assign");
        assert_eq!(label_bin(parse_quote!(a &= b)), "bitand_assign");
        assert_eq!(label_bin(parse_quote!(a |= b)), "bitor_assign");
        assert_eq!(label_bin(parse_quote!(a <<= b)), "shl_assign");
        assert_eq!(label_bin(parse_quote!(a >>= b)), "shr_assign");
    }

    #[test]
    fn unary_and_fallback_labels() {
        assert_eq!(label_un(parse_quote!(*a)), "deref");
        assert_eq!(label_un(parse_quote!(!a)), "not");
        assert_eq!(label_un(parse_quote!(-a)), "neg");
        assert_eq!(bin_op_label_debug("TotallyUnknown"), "other");
        assert_eq!(un_op_label_debug("TotallyUnknown"), "other");
    }
}
