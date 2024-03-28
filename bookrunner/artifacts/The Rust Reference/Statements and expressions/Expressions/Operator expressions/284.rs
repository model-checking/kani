// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let x = false || true; // true
let y = false && panic!(); // false, doesn't evaluate `panic!()`
}