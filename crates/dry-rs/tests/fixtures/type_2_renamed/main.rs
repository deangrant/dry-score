//! Type-2 fixture: identical structure with renamed identifiers.

fn alpha_path(value: i32, weight: i32) -> i32 {
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

fn beta_path(amount: i32, multiplier: i32) -> i32 {
    let mut running = amount;
    if running < 0 {
        running = 0 - running;
    }
    let product = running * multiplier;
    if product > 100 {
        return product - 10;
    }
    product + 1
}
