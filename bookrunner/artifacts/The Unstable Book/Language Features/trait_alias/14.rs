// compile-flags: --edition 2015
#![allow(unused)]
#![feature(trait_alias)]

trait Foo = std::fmt::Debug + Send;
trait Bar = Foo + Sync;

// Use trait alias as bound on type parameter.
fn foo<T: Foo>(v: &T) {
    println!("{:?}", v);
}

pub fn main() {
    foo(&1);

    // Use trait alias for trait objects.
    let a: &Bar = &123;
    println!("{:?}", a);
    let b = Box::new(456) as Box<dyn Foo>;
    println!("{:?}", b);
}