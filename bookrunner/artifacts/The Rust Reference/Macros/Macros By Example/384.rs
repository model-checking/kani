// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
pub mod inner {
    #[macro_export]
    macro_rules! call_foo {
        () => { $crate::inner::foo() };
    }

    pub fn foo() {}
}
}