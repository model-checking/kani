// compile-flags: --edition 2021
// kani-flags: --enable-unstable --cbmc-args --unwind 4
#![allow(unused)]
fn main() {
let v = &["apples", "cake", "coffee"];

for text in v {
    println!("I like {}.", text);
}
}