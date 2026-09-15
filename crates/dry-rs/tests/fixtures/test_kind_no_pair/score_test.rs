//! Test-kind twin of `score_prod` (must not pair with production).

#[test]
fn score_test(input: i32, factor: i32) -> i32 {
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
