// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
macro_rules! m {
    ($item: item) => { $item $item }
}

m!(use std as _;);
// This expands to:
// use std as _;
// use std as _;
}