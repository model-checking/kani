#[macro_use]
extern crate proptest_derive;

#[derive(Arbitrary)] //~ `Foo` doesn't implement `std::fmt::Debug` [E0277]
struct Foo { x: usize }
