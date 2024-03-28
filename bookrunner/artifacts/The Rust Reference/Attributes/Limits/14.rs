// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
#![recursion_limit = "4"]

fn main() {
macro_rules! a {
    () => { a!(1); };
    (1) => { a!(2); };
    (2) => { a!(3); };
    (3) => { a!(4); };
    (4) => { };
}

// This fails to expand because it requires a recursion depth greater than 4.
a!{}
}