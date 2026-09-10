//! Type-3 fixture: shared structure with a small statement-level edit.

fn near_left(value: i32, weight: i32) -> i32 {
    let mut acc = value;
    if acc < 0 {
        acc = 0 - acc;
    }
    let product = acc * weight;
    if product > 100 {
        return product - 10;
    }
    product + 1
}

fn near_right(value: i32, weight: i32) -> i32 {
    let mut acc = value;
    if acc < 0 {
        acc = 0 - acc;
    }
    let product = acc * weight;
    if product > 100 {
        return product - 10;
    }
    let bonus = 2;
    product + bonus
}
