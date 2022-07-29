// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Point(i32, i32);
let p = Point(10, 11);
let px: i32 = match p { Point(x, _) => x };
}