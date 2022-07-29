// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Point {x: i32, y: i32}
let p = Point {x: 10, y: 11};
let px: i32 = p.x;
}