// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
#![recursion_limit = "1"]

// This fails because it requires two recursive steps to auto-dereference.
fn main() {
(|_: &u8| {})(&&&1);
}