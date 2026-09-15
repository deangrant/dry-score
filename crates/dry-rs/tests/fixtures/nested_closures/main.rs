//! Nested closures with identical bodies (Type-1 via nested forms).

fn with_closures() -> i32 {
    let left = |n: i32| -> i32 {
        let mut acc = n;
        if acc < 0 {
            acc = 0 - acc;
        }
        acc + 1
    };
    let right = |n: i32| -> i32 {
        let mut acc = n;
        if acc < 0 {
            acc = 0 - acc;
        }
        acc + 1
    };
    left(3) + right(4)
}
