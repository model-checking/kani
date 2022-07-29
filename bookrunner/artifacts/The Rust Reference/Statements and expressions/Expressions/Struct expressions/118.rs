// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Gamma;
let a = Gamma;  // Gamma unit value.
let b = Gamma{};  // Exact same value as `a`.
}