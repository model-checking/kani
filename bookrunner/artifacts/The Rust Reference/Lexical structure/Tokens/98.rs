// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
macro_rules! blackhole { ($tt:tt) => () }

blackhole!("string"suffix); // OK
}