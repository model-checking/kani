// compile-flags: --edition 2021
// kani-flags: --enable-unstable --cbmc-args --unwind 0
#![allow(unused)]
fn main() {
enum List<T> {
    Nil,
    Cons(T, Box<List<T>>)
}

let a: List<i32> = List::Cons(7, Box::new(List::Cons(13, Box::new(List::Nil))));
}