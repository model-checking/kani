// compile-flags: --edition 2021
#![allow(unused)]
// Tells the rustfmt tool to not format the following element.
fn main() {
#[rustfmt::skip]
struct S {
}

// Controls the "cyclomatic complexity" threshold for the clippy tool.
#[clippy::cyclomatic_complexity = "100"]
pub fn f() {}
}