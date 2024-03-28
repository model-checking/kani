// compile-flags: --edition 2015
#![allow(unused)]
#![feature(unsized_fn_params)]

trait Foo {
    fn foo(self) {}
}

impl<T: ?Sized> Foo for T {}

fn main() {
    let slice: Box<[i32]> = Box::new([1, 2, 3]);
    <[i32] as Foo>::foo(*slice);
}