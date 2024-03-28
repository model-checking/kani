// compile-flags: --edition 2015
#![allow(unused)]
#![feature(negative_impls)]
fn main() {
trait DerefMut { }
impl<T: ?Sized> !DerefMut for &T { }
}