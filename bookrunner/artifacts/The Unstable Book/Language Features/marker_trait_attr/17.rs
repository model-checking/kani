// compile-flags: --edition 2015
#![allow(unused)]
#![feature(marker_trait_attr)]

fn main() {
#[marker] trait CheapToClone: Clone {}

impl<T: Copy> CheapToClone for T {}

// These could potentially overlap with the blanket implementation above,
// so are only allowed because CheapToClone is a marker trait.
impl<T: CheapToClone, U: CheapToClone> CheapToClone for (T, U) {}
impl<T: CheapToClone> CheapToClone for std::ops::Range<T> {}

fn cheap_clone<T: CheapToClone>(t: T) -> T {
    t.clone()
}
}