// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
enum Link {
    Next(Box<Link>),
    None,
}
}