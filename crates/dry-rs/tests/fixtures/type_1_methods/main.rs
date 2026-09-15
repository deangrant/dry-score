//! Type-1 fixture: identical impl methods.

struct Counter {
    n: i32,
}

impl Counter {
    fn score_left(&self, input: i32, factor: i32) -> i32 {
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

    fn score_right(&self, input: i32, factor: i32) -> i32 {
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
}
