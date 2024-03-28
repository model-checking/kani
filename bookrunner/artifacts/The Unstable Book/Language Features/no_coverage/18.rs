// compile-flags: --edition 2015
#![allow(unused)]
#![feature(no_coverage)]

// `foo()` will get coverage instrumentation (by default)
fn main() {
fn foo() {
  // ...
}

#[no_coverage]
fn bar() {
  // ...
}
}