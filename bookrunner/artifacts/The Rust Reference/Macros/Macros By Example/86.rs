// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
macro_rules! foo {
    ($l:expr) => { bar!($l); }
// ERROR:               ^^ no rules expected this token in macro call
}

macro_rules! bar {
    (3) => {}
}

foo!(3);
}