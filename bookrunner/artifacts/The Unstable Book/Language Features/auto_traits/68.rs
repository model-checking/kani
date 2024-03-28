// compile-flags: --edition 2015
#![allow(unused)]
fn main() {
struct List<T> {
  data: T,
  next: Option<Box<List<T>>>,
}

unsafe impl<T> Send for List<T>
where
  T: Send, // from the field `data`
  Option<Box<List<T>>>: Send, // from the field `next`
{ }
}