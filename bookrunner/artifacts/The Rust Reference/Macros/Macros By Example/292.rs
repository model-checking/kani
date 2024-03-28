// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[macro_use]
mod inner {
    macro_rules! m {
        () => {};
    }
}

m!();
}