//! Operator label helpers.

use syn::{BinOp, UnOp};

/// Stable label for a binary operator.
#[must_use]
pub const fn bin_op_label(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add(_) => "add",
        BinOp::Sub(_) => "sub",
        BinOp::Mul(_) => "mul",
        BinOp::Div(_) => "div",
        BinOp::Rem(_) => "rem",
        BinOp::And(_) => "and",
        BinOp::Or(_) => "or",
        BinOp::BitXor(_) => "bitxor",
        BinOp::BitAnd(_) => "bitand",
        BinOp::BitOr(_) => "bitor",
        BinOp::Shl(_) => "shl",
        BinOp::Shr(_) => "shr",
        BinOp::Eq(_) => "eq",
        BinOp::Ne(_) => "ne",
        BinOp::Lt(_) => "lt",
        BinOp::Le(_) => "le",
        BinOp::Gt(_) => "gt",
        BinOp::Ge(_) => "ge",
        BinOp::AddAssign(_) => "add_assign",
        BinOp::SubAssign(_) => "sub_assign",
        BinOp::MulAssign(_) => "mul_assign",
        BinOp::DivAssign(_) => "div_assign",
        BinOp::RemAssign(_) => "rem_assign",
        BinOp::BitXorAssign(_) => "bitxor_assign",
        BinOp::BitAndAssign(_) => "bitand_assign",
        BinOp::BitOrAssign(_) => "bitor_assign",
        BinOp::ShlAssign(_) => "shl_assign",
        BinOp::ShrAssign(_) => "shr_assign",
        _ => "other",
    }
}

/// Stable label for a unary operator.
#[must_use]
pub const fn un_op_label(op: &UnOp) -> &'static str {
    match op {
        UnOp::Deref(_) => "deref",
        UnOp::Not(_) => "not",
        UnOp::Neg(_) => "neg",
        _ => "other",
    }
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
    fn unary_labels() {
        assert_eq!(label_un(parse_quote!(*a)), "deref");
        assert_eq!(label_un(parse_quote!(!a)), "not");
        assert_eq!(label_un(parse_quote!(-a)), "neg");
    }
}
