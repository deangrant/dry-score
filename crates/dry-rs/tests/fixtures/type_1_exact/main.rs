//! Type-1 fixture: identical structure and identical identifiers.

fn score_left(input: i32, factor: i32) -> i32 {
    let mut total = input;
    if total < 0 {
        total = 0 - total;
    }
    let scaled = total * factor;
    if scaled > 100 {
        return scaled - 10;
    }
    scaled + 1
}

fn score_right(input: i32, factor: i32) -> i32 {
    let mut total = input;
    if total < 0 {
        total = 0 - total;
    }
    let scaled = total * factor;
    if scaled > 100 {
        return scaled - 10;
    }
    scaled + 1
}
