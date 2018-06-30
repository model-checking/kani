#![feature(never_type)]

#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0004]
enum Void {}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0004]
enum FooBar {}
