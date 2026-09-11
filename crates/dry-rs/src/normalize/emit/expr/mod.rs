//! Expression emission helpers.

mod control;
mod misc;
mod primary;
mod wrap;

use syn::Expr;

use crate::normalize::placeholders::PlaceholderMap;
use dry_core::NormNode;

use super::lit_label;
use super::mac::emit_macro;
use control::{
    emit_assign, emit_async, emit_await, emit_for, emit_if, emit_loop, emit_match, emit_while,
};
use misc::{
    emit_break, emit_cast, emit_const_block, emit_continue, emit_infer, emit_let, emit_range,
    emit_raw_addr, emit_repeat, emit_struct, emit_try_block, emit_unsafe, emit_yield,
};
use primary::{
    emit_binary, emit_call, emit_closure, emit_field, emit_list, emit_method_call, emit_path,
    emit_return, emit_unary,
};

pub(super) use wrap::emit_range_like;

/// Emits a normalized tree for an expression.
pub fn emit_expr(expr: &Expr, placeholders: &mut PlaceholderMap) -> NormNode {
    try_emit_ops(expr, placeholders)
        .or_else(|| try_emit_atom(expr, placeholders))
        .or_else(|| try_emit_wrap(expr, placeholders))
        .or_else(|| try_emit_callish(expr, placeholders))
        .or_else(|| try_emit_aggregate(expr, placeholders))
        .or_else(|| try_emit_branch(expr, placeholders))
        .or_else(|| try_emit_loopish(expr, placeholders))
        .or_else(|| try_emit_asyncish(expr, placeholders))
        .or_else(|| try_emit_misc(expr, placeholders))
        .unwrap_or_else(|| NormNode::leaf("expr_other"))
}

fn try_emit_ops(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Binary(bin) => emit_binary(bin, placeholders),
        Expr::Unary(unary) => emit_unary(unary, placeholders),
        Expr::Assign(assign) => emit_assign(assign, placeholders),
        _ => return None,
    })
}

fn try_emit_atom(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    try_emit_path_lit(expr, placeholders).or_else(|| try_emit_control_atom(expr, placeholders))
}

fn try_emit_path_lit(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Path(path) => emit_path(path, placeholders),
        Expr::Lit(lit) => NormNode::leaf(lit_label(&lit.lit)),
        _ => return None,
    })
}

fn try_emit_control_atom(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Return(ret) => emit_return(ret, placeholders),
        Expr::Infer(_) => emit_infer(),
        Expr::Continue(_) => emit_continue(),
        _ => return try_emit_macro(expr, placeholders),
    })
}

fn try_emit_macro(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    match expr {
        Expr::Macro(mac) => Some(emit_macro(&mac.mac, placeholders, emit_expr)),
        _ => None,
    }
}

fn try_emit_wrap(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    try_emit_refish(expr, placeholders)
        .or_else(|| try_emit_groupish(expr, placeholders))
        .or_else(|| try_emit_closure(expr, placeholders))
}

fn try_emit_refish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Reference(reference) => {
            NormNode::branch("ref", vec![emit_expr(&reference.expr, placeholders)])
        }
        Expr::RawAddr(raw) => emit_raw_addr(raw, placeholders),
        Expr::Try(expr_try) => {
            NormNode::branch("try", vec![emit_expr(&expr_try.expr, placeholders)])
        }
        _ => return None,
    })
}

fn try_emit_groupish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Paren(paren) => emit_expr(&paren.expr, placeholders),
        Expr::Group(group) => emit_expr(&group.expr, placeholders),
        _ => return None,
    })
}

fn try_emit_closure(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    match expr {
        Expr::Closure(closure) => Some(emit_closure(closure, placeholders)),
        _ => None,
    }
}

fn try_emit_callish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Call(call) => emit_call(call, placeholders),
        Expr::MethodCall(method) => emit_method_call(method, placeholders),
        _ => return try_emit_fieldish(expr, placeholders),
    })
}

fn try_emit_fieldish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    match expr {
        Expr::Field(field) => Some(emit_field(field, placeholders)),
        _ => try_emit_index(expr, placeholders),
    }
}

fn try_emit_index(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    match expr {
        Expr::Index(index) => Some(NormNode::branch(
            "index",
            vec![
                emit_expr(&index.expr, placeholders),
                emit_expr(&index.index, placeholders),
            ],
        )),
        _ => None,
    }
}

fn try_emit_aggregate(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    try_emit_collection(expr, placeholders).or_else(|| try_emit_struct_range(expr, placeholders))
}

fn try_emit_collection(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Tuple(tuple) => emit_list("tuple", &tuple.elems, placeholders),
        Expr::Array(array) => emit_list("array", &array.elems, placeholders),
        Expr::Repeat(repeat) => emit_repeat(repeat, placeholders),
        _ => return None,
    })
}

fn try_emit_struct_range(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Struct(expr_struct) => emit_struct(expr_struct, placeholders),
        Expr::Range(range) => emit_range(range, placeholders),
        _ => return None,
    })
}

fn try_emit_branch(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::If(expr_if) => emit_if(expr_if, placeholders),
        Expr::Match(expr_match) => emit_match(expr_match, placeholders),
        _ => return try_emit_block_expr(expr, placeholders),
    })
}

fn try_emit_block_expr(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    try_emit_plain_block(expr, placeholders).or_else(|| try_emit_special_block(expr, placeholders))
}

fn try_emit_plain_block(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    match expr {
        Expr::Block(expr_block) => Some(super::emit_block(&expr_block.block, placeholders)),
        Expr::Unsafe(expr_unsafe) => Some(emit_unsafe(expr_unsafe, placeholders)),
        _ => None,
    }
}

fn try_emit_special_block(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    match expr {
        Expr::TryBlock(try_block) => Some(emit_try_block(try_block, placeholders)),
        Expr::Const(expr_const) => Some(emit_const_block(expr_const, placeholders)),
        _ => None,
    }
}

fn try_emit_loopish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::While(expr_while) => emit_while(expr_while, placeholders),
        Expr::ForLoop(for_loop) => emit_for(for_loop, placeholders),
        _ => return try_emit_loop(expr, placeholders),
    })
}

fn try_emit_loop(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    match expr {
        Expr::Loop(expr_loop) => Some(emit_loop(expr_loop, placeholders)),
        _ => None,
    }
}

fn try_emit_asyncish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Async(expr_async) => emit_async(expr_async, placeholders),
        Expr::Await(expr_await) => emit_await(expr_await, placeholders),
        _ => return None,
    })
}

fn try_emit_misc(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    try_emit_break_cast(expr, placeholders).or_else(|| try_emit_let_yield(expr, placeholders))
}

fn try_emit_break_cast(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Break(expr_break) => emit_break(expr_break, placeholders),
        Expr::Cast(cast) => emit_cast(cast, placeholders),
        _ => return None,
    })
}

fn try_emit_let_yield(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    // dry-rs:ignore. CC-split expr dispatch shells; parallel shape is intentional.
    Some(match expr {
        Expr::Let(expr_let) => emit_let(expr_let, placeholders),
        Expr::Yield(expr_yield) => emit_yield(expr_yield, placeholders),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::placeholders::PlaceholderMap;
    use syn::parse_quote;

    fn sample_exprs() -> Vec<syn::Expr> {
        vec![
            parse_quote!(a + b),
            parse_quote!(x),
            parse_quote!(f(1)),
            parse_quote!(a.foo(1)),
            parse_quote!(a.b),
            parse_quote!((1, 2)),
            parse_quote!(if true {
                1
            }),
            parse_quote!(while false {
                break;
            }),
            parse_quote!(for x in xs {
                x;
            }),
            parse_quote!(loop {
                break;
            }),
            parse_quote!(match x {
                1 => 2,
                _ => 3,
            }),
            parse_quote!(async { 1 }),
            parse_quote!(x.await),
            parse_quote!(println!("{}", 1)),
            parse_quote!(&x),
            parse_quote!((x)),
            parse_quote!(x?),
            parse_quote!(|x| x),
            parse_quote!(xs[0]),
            parse_quote!([1, 2]),
            parse_quote!({
                let x = 1;
                x
            }),
            parse_quote!(break),
            parse_quote!(continue),
            parse_quote!(x as u32),
            parse_quote!(1..n),
            parse_quote!(Point { x: 1 }),
            parse_quote!([0; 3]),
            parse_quote!(unsafe { 1 }),
            parse_quote!(const { 1 }),
            parse_quote!(&raw const x),
        ]
    }

    #[test]
    fn emit_expr_covers_families() {
        let mut placeholders = PlaceholderMap::default();
        for sample in sample_exprs() {
            let node = emit_expr(&sample, &mut placeholders);
            assert_ne!(node.label, "expr_other");
            assert!(!node.label.starts_with("expr:"));
        }
    }

    #[test]
    fn expr_other_fallback_is_stable() {
        let verbatim = Expr::Verbatim(proc_macro2::TokenStream::new());
        let mut p = PlaceholderMap::default();
        assert_eq!(emit_expr(&verbatim, &mut p).label, "expr_other");
    }

    #[test]
    fn let_in_if_condition_emits_let() {
        let mut p = PlaceholderMap::default();
        let expr: syn::Expr = parse_quote!(if let Some(x) = y {
            x
        });
        let node = emit_expr(&expr, &mut p);
        assert_eq!(node.label, "if");
        assert_eq!(node.children[0].label, "let");
    }
}
