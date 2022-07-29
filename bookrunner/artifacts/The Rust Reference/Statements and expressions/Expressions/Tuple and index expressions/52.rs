// compile-flags: --edition 2021
#![allow(unused)]
// Indexing a tuple
fn main() {
let pair = ("a string", 2);
assert_eq!(pair.1, 2);

// Indexing a tuple struct
struct Point(f32, f32);
let point = Point(1.0, 0.0);
assert_eq!(point.0, 1.0);
assert_eq!(point.1, 0.0);
}