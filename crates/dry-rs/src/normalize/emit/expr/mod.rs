//! Expression emission helpers.

mod control;
mod primary;

use syn::Expr;
use syn::spanned::Spanned;

use crate::normalize::placeholders::PlaceholderMap;
use crate::normalize::tree::NormNode;

use control::{
    emit_assign, emit_async, emit_await, emit_for, emit_if, emit_loop, emit_match, emit_while,
};
use primary::{
    emit_binary, emit_call, emit_closure, emit_field, emit_list, emit_method_call, emit_path,
    emit_return, emit_unary, lit_label,
};

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
        .unwrap_or_else(|| NormNode::leaf(format!("expr:{}", expr.span().start().line)))
}

fn try_emit_ops(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match expr {
        Expr::Binary(bin) => emit_binary(bin, placeholders),
        Expr::Unary(unary) => emit_unary(unary, placeholders),
        Expr::Assign(assign) => emit_assign(assign, placeholders),
        _ => return None,
    })
}

fn try_emit_atom(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match expr {
        Expr::Path(path) => emit_path(path, placeholders),
        Expr::Lit(lit) => NormNode::leaf(lit_label(&lit.lit)),
        Expr::Return(ret) => emit_return(ret, placeholders),
        _ => return try_emit_macro(expr),
    })
}

fn try_emit_macro(expr: &Expr) -> Option<NormNode> {
    match expr {
        Expr::Macro(_) => Some(NormNode::leaf("expr_macro")),
        _ => None,
    }
}

fn try_emit_wrap(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match expr {
        Expr::Reference(reference) => {
            NormNode::branch("ref", vec![emit_expr(&reference.expr, placeholders)])
        }
        Expr::Paren(paren) => emit_expr(&paren.expr, placeholders),
        Expr::Try(expr_try) => {
            NormNode::branch("try", vec![emit_expr(&expr_try.expr, placeholders)])
        }
        _ => return try_emit_closure(expr, placeholders),
    })
}

fn try_emit_closure(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    match expr {
        Expr::Closure(closure) => Some(emit_closure(closure, placeholders)),
        _ => None,
    }
}

fn try_emit_callish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match expr {
        Expr::Call(call) => emit_call(call, placeholders),
        Expr::MethodCall(method) => emit_method_call(method, placeholders),
        _ => return try_emit_fieldish(expr, placeholders),
    })
}

fn try_emit_fieldish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    match expr {
        Expr::Field(field) => Some(emit_field(field, placeholders)),
        _ => try_emit_index(expr, placeholders),
    }
}

fn try_emit_index(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
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
    Some(match expr {
        Expr::Tuple(tuple) => emit_list("tuple", &tuple.elems, placeholders),
        Expr::Array(array) => emit_list("array", &array.elems, placeholders),
        _ => return None,
    })
}

fn try_emit_branch(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match expr {
        Expr::If(expr_if) => emit_if(expr_if, placeholders),
        Expr::Match(expr_match) => emit_match(expr_match, placeholders),
        _ => return try_emit_block_expr(expr, placeholders),
    })
}

fn try_emit_block_expr(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    match expr {
        Expr::Block(expr_block) => Some(super::emit_block(&expr_block.block, placeholders)),
        _ => None,
    }
}

fn try_emit_loopish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match expr {
        Expr::While(expr_while) => emit_while(expr_while, placeholders),
        Expr::ForLoop(for_loop) => emit_for(for_loop, placeholders),
        _ => return try_emit_loop(expr, placeholders),
    })
}

fn try_emit_loop(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    match expr {
        Expr::Loop(expr_loop) => Some(emit_loop(expr_loop, placeholders)),
        _ => None,
    }
}

fn try_emit_asyncish(expr: &Expr, placeholders: &mut PlaceholderMap) -> Option<NormNode> {
    Some(match expr {
        Expr::Async(expr_async) => emit_async(expr_async, placeholders),
        Expr::Await(expr_await) => emit_await(expr_await, placeholders),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::placeholders::PlaceholderMap;
    use syn::parse_quote;

    #[test]
    fn emit_expr_covers_families() {
        let mut p = PlaceholderMap::default();
        let samples: Vec<syn::Expr> = vec![
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
        ];
        for sample in samples {
            let _ = emit_expr(&sample, &mut p);
        }
    }
}
