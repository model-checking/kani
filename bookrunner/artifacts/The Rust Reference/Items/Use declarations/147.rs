// compile-flags: --edition 2021
#![allow(unused)]
// use std::fs; // Error, this is ambiguous.
use ::std::fs;  // Imports from the `std` crate, not the module below.
use self::std::fs as self_fs;  // Imports the module below.

mod std {
    pub mod fs {}
}
fn main() {}