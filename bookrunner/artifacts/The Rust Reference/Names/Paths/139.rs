// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
mod ops {
    pub struct Range<T> {f1: T}
    pub trait Index<T> {}
    pub struct Example<'a> {f1: &'a i32}
}
struct S;
impl ops::Index<ops::Range<usize>> for S { /*...*/ }
fn i<'a>() -> impl Iterator<Item = ops::Example<'a>> {
    // ...
   const EXAMPLE: Vec<ops::Example<'static>> = Vec::new();
   EXAMPLE.into_iter()
}
type G = std::boxed::Box<dyn std::ops::FnOnce(isize) -> isize>;
}