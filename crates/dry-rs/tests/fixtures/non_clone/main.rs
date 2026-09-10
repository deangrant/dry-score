//! Non-clone fixture: similar names, different control flow.

fn compute_sum(values: &[i32]) -> i32 {
    let mut total = 0;
    for value in values {
        total += *value;
    }
    if total < 0 {
        return 0;
    }
    total
}

fn compute_product(values: &[i32]) -> i32 {
    let mut total = 1;
    for value in values {
        if *value == 0 {
            return 0;
        }
        total *= *value;
    }
    total
}
