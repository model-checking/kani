// compile-flags: --edition 2015
#![allow(unused)]
fn add_one(x: i32) -> i32 { x + 1 }
const MY_COMPUTATION: i32 = 1 + 2 * 3 / 4;

fn main() {
    let x = add_one(MY_COMPUTATION);
}