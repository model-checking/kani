// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[must_use]
fn five() -> i32 { 5i32 }

// Violates the unused_must_use lint.
five();
}