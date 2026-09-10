//! Operator label helpers.

use syn::{BinOp, UnOp};

/// Stable label for a binary operator.
#[must_use]
pub const fn bin_op_label(op: &BinOp) -> &'static str {
    let step1 = or_label(bin_arith_label(op), bin_logic_label(op));
    let step2 = or_label(step1, bin_bit_label(op));
    let step3 = or_label(step2, bin_cmp_label(op));
    label_or_other(or_label(step3, bin_assign_label(op)))
}

const fn label_or_other(label: Option<&'static str>) -> &'static str {
    match label {
        Some(label) => label,
        None => "other",
    }
}

const fn or_label(
    first: Option<&'static str>,
    second: Option<&'static str>,
) -> Option<&'static str> {
    match first {
        Some(label) => Some(label),
        None => second,
    }
}

const fn bin_arith_label(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::Add(_) => Some("add"),
        BinOp::Sub(_) => Some("sub"),
        BinOp::Mul(_) => Some("mul"),
        _ => bin_arith_div_rem(op),
    }
}

const fn bin_arith_div_rem(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::Div(_) => Some("div"),
        BinOp::Rem(_) => Some("rem"),
        _ => None,
    }
}

const fn bin_logic_label(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::And(_) => Some("and"),
        BinOp::Or(_) => Some("or"),
        _ => None,
    }
}

const fn bin_bit_label(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::BitXor(_) => Some("bitxor"),
        BinOp::BitAnd(_) => Some("bitand"),
        BinOp::BitOr(_) => Some("bitor"),
        _ => bin_shift_label(op),
    }
}

const fn bin_shift_label(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::Shl(_) => Some("shl"),
        BinOp::Shr(_) => Some("shr"),
        _ => None,
    }
}

const fn bin_cmp_label(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::Eq(_) => Some("eq"),
        BinOp::Ne(_) => Some("ne"),
        BinOp::Lt(_) => Some("lt"),
        _ => bin_cmp_ordered(op),
    }
}

const fn bin_cmp_ordered(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::Le(_) => Some("le"),
        BinOp::Gt(_) => Some("gt"),
        BinOp::Ge(_) => Some("ge"),
        _ => None,
    }
}

const fn bin_assign_label(op: &BinOp) -> Option<&'static str> {
    or_label(bin_assign_arith_label(op), bin_assign_bit_label(op))
}

const fn bin_assign_arith_label(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::AddAssign(_) => Some("add_assign"),
        BinOp::SubAssign(_) => Some("sub_assign"),
        BinOp::MulAssign(_) => Some("mul_assign"),
        _ => bin_assign_div_rem(op),
    }
}

const fn bin_assign_div_rem(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::DivAssign(_) => Some("div_assign"),
        BinOp::RemAssign(_) => Some("rem_assign"),
        _ => None,
    }
}

const fn bin_assign_bit_label(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::BitXorAssign(_) => Some("bitxor_assign"),
        BinOp::BitAndAssign(_) => Some("bitand_assign"),
        BinOp::BitOrAssign(_) => Some("bitor_assign"),
        _ => bin_assign_shift(op),
    }
}

const fn bin_assign_shift(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::ShlAssign(_) => Some("shl_assign"),
        BinOp::ShrAssign(_) => Some("shr_assign"),
        _ => None,
    }
}

/// Stable label for a unary operator.
#[must_use]
pub const fn un_op_label(op: &UnOp) -> &'static str {
    if matches!(op, UnOp::Deref(_)) {
        return "deref";
    }
    if matches!(op, UnOp::Not(_)) {
        return "not";
    }
    "neg"
}

#[cfg(test)]
mod tests {
    use super::{bin_op_label, label_or_other, un_op_label};
    use syn::parse_quote;

    #[test]
    fn labels_cover_common_ops() {
        let cases: &[(&str, &str)] = &[
            ("a + b", "add"),
            ("a - b", "sub"),
            ("a * b", "mul"),
            ("a / b", "div"),
            ("a % b", "rem"),
            ("a && b", "and"),
            ("a || b", "or"),
            ("a ^ b", "bitxor"),
            ("a & b", "bitand"),
            ("a | b", "bitor"),
            ("a << b", "shl"),
            ("a >> b", "shr"),
            ("a == b", "eq"),
            ("a != b", "ne"),
            ("a < b", "lt"),
            ("a <= b", "le"),
            ("a > b", "gt"),
            ("a >= b", "ge"),
            ("a += b", "add_assign"),
            ("a -= b", "sub_assign"),
            ("a *= b", "mul_assign"),
            ("a /= b", "div_assign"),
            ("a %= b", "rem_assign"),
            ("a ^= b", "bitxor_assign"),
            ("a &= b", "bitand_assign"),
            ("a |= b", "bitor_assign"),
            ("a <<= b", "shl_assign"),
            ("a >>= b", "shr_assign"),
        ];
        for (src, expected) in cases {
            #[expect(clippy::expect_used, reason = "fixed operator corpus strings")]
            let expr: syn::ExprBinary = syn::parse_str(src).expect("parse binop");
            assert_eq!(bin_op_label(&expr.op), *expected, "{src}");
        }

        let deref: syn::ExprUnary = parse_quote!(*x);
        assert_eq!(un_op_label(&deref.op), "deref");
        let not: syn::ExprUnary = parse_quote!(!x);
        assert_eq!(un_op_label(&not.op), "not");
        let neg: syn::ExprUnary = parse_quote!(-x);
        assert_eq!(un_op_label(&neg.op), "neg");
        assert_eq!(label_or_other(None), "other");
        assert_eq!(label_or_other(Some("add")), "add");
    }
}
