// compile-flags: --edition 2021
// kani-flags: --enable-unstable --cbmc-args --unwind 0
#![allow(unused)]
fn main() {
let pi: Result<f32, _> = "3.14".parse();
let log_pi = pi.unwrap_or(1.0).log(2.72);
assert!(1.14 < log_pi && log_pi < 1.15)
}