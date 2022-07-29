// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[must_use]
fn five() -> i32 { 5i32 }

// Does not violate the unused_must_use lint.
let _ = five();
}