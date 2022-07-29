// compile-flags: --edition 2021
#![allow(unused)]
// compiles OK
fn main() {
macro_rules! foo {
    ($l:tt) => { bar!($l); }
}

macro_rules! bar {
    (3) => {}
}

foo!(3);
}