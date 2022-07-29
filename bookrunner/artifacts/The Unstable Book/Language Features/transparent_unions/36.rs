// compile-flags: --edition 2015
#![allow(unused)]
#![feature(transparent_unions)]

// This (non-transparent) union is already valid in stable Rust:
fn main() {
pub union GoodUnion {
    pub nothing: (),
}

// Error: transparent union needs exactly one non-zero-sized field, but has 0
// #[repr(transparent)]
// pub union BadUnion {
//     pub nothing: (),
// }
}