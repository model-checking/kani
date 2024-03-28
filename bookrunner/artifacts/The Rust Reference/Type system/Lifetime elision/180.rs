// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Foo;
struct Bar;
struct Baz;
fn somefunc(a: &Foo, b: &Bar, c: &Baz) -> usize {42}
// Resolved as `fn<'a>(&'a str) -> &'a str`.
const RESOLVED_SINGLE: fn(&str) -> &str = |x| x;

// Resolved as `Fn<'a, 'b, 'c>(&'a Foo, &'b Bar, &'c Baz) -> usize`.
const RESOLVED_MULTIPLE: &dyn Fn(&Foo, &Bar, &Baz) -> usize = &somefunc;
}