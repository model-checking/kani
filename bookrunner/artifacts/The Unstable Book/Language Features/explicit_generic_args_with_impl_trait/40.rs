// kani-check-fail
// compile-flags: --edition 2015
#![allow(unused)]
#![feature(explicit_generic_args_with_impl_trait)]

fn foo<T: ?Sized>(_f: impl AsRef<T>) {}
fn bar<T: ?Sized, F: AsRef<T>>(_f: F) {}

fn main() {
    bar::<str, _>("".to_string()); // Okay
    bar::<str, String>("".to_string()); // Okay

    foo::<str>("".to_string()); // Okay
    foo::<str, String>("".to_string()); // Error, you cannot specify `impl Trait` explicitly
}