// compile-flags: --edition 2021
#![allow(unused)]
mod a {
    pub fn foo() {}
}
mod b {
    pub fn foo() {
        super::a::foo(); // call a's foo function
    }
}
fn main() {}