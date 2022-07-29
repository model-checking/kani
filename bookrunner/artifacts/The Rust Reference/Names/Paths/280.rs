// compile-flags: --edition 2021
#![allow(unused)]
fn foo() {}
mod a {
    fn bar() {
        crate::foo();
    }
}
fn main() {}