// compile-flags: --edition 2018
#![allow(unused)]
#![feature(trait_upcasting)]
#![allow(incomplete_features)]

fn main() {
trait Foo {}

trait Bar: Foo {}

impl Foo for i32 {}

impl<T: Foo + ?Sized> Bar for T {}

let bar: &dyn Bar = &123;
let foo: &dyn Foo = bar;
}