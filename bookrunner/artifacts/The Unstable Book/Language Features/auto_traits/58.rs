// compile-flags: --edition 2015
#![allow(unused)]
fn main() {
struct List<T> {
  data: T,
  next: Option<Box<List<T>>>,
}
}