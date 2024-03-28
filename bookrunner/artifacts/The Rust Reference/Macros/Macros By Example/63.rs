// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
macro_rules! ambiguity {
    ($($i:ident)* $j:ident) => { };
}

ambiguity!(error); // Error: local ambiguity
}