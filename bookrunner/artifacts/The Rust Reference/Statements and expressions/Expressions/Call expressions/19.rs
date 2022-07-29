// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
fn add(x: i32, y: i32) -> i32 { 0 }
let three: i32 = add(1i32, 2i32);
let name: &'static str = (|| "Rust")();
}