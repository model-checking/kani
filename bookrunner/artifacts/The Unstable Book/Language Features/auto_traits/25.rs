// compile-flags: --edition 2015
#![allow(unused)]
#![feature(negative_impls)]
#![feature(auto_traits)]

auto trait Valid {}

struct True;
struct False;

impl !Valid for False {}

struct MaybeValid<T>(T);

fn must_be_valid<T: Valid>(_t: T) { }

fn main() {
    // works
    must_be_valid( MaybeValid(True) );

    // compiler error - trait bound not satisfied
    // must_be_valid( MaybeValid(False) );
}