// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
trait Container {
    type E;
    fn empty() -> Self;
    fn insert(&mut self, elem: Self::E);
}
}