// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
fn returns_closure() -> Box<dyn Fn(i32) -> i32> {
    Box::new(|x| x + 1)
}
}