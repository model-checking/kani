#[macro_use]
extern crate proptest_derive;

#[derive(Arbitrary)] //~ ERROR: [proptest_derive, E0002]
union Foo { x: usize }