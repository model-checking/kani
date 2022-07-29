// compile-flags: --edition 2015
#![allow(unused)]
#![feature(inline_const)]

fn add_one(x: i32) -> i32 { x + 1 }
fn main() {
    let x = add_one(const { 1 + 2 * 3 / 4 });
}