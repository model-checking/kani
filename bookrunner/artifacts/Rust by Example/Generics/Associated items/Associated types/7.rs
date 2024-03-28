// compile-flags: --edition 2015
#![allow(unused)]
// `A` and `B` are defined in the trait via the `type` keyword.
// (Note: `type` in this context is different from `type` when used for
// aliases).
fn main() {
trait Contains {
    type A;
    type B;

    // Updated syntax to refer to these new types generically.
    fn contains(&self, &Self::A, &Self::B) -> bool;
}
}