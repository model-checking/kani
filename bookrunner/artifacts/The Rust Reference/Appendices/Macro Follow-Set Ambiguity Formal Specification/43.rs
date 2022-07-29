// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
macro_rules! i_am_an_mbe {
    (start $foo:expr $($i:ident),* end) => ($foo)
}
}