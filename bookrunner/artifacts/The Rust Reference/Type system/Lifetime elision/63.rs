// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
// The following examples show situations where it is not allowed to elide the
// lifetime parameter.

fn main() {
trait Example {
// Cannot infer, because there are no parameters to infer from.
fn get_str() -> &str;                                 // ILLEGAL

// Cannot infer, ambiguous if it is borrowed from the first or second parameter.
fn frob(s: &str, t: &str) -> &str;                    // ILLEGAL
}
}