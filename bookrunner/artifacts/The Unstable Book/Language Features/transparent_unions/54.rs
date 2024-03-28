// compile-flags: --edition 2015
#![allow(unused)]
#![feature(transparent_unions)]

// This union has the same representation as `T`.
fn main() {
#[repr(transparent)]
pub union GenericUnion<T: Copy> { // Unions with non-`Copy` fields are unstable.
    pub field: T,
    pub nothing: (),
}

// This is okay even though `()` is a zero-sized type.
pub const THIS_IS_OKAY: GenericUnion<()> = GenericUnion { field: () };
}