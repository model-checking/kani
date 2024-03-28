// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
(0..10).collect::<Vec<_>>();
Vec::<u8>::with_capacity(1024);
}