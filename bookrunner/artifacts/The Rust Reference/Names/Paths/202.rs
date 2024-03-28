// compile-flags: --edition 2021
#![allow(unused)]
fn foo() {}
fn bar() {
    self::foo();
}
fn main() {}