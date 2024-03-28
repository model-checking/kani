// compile-flags: --edition 2015
#![allow(unused)]
#![feature(transparent_unions)]

// This union has the same representation as `f32`.
fn main() {
#[repr(transparent)]
union SingleFieldUnion {
    field: f32,
}

// This union has the same representation as `usize`.
#[repr(transparent)]
union MultiFieldUnion {
    field: usize,
    nothing: (),
}
}