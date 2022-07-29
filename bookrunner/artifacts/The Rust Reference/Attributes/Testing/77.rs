// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[test]
#[should_panic(expected = "values don't match")]
fn mytest() {
    assert_eq!(1, 2, "values don't match");
}
}