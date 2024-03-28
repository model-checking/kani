// compile-flags: --edition 2015
#![allow(unused)]
#![feature(concat_idents)]

fn main() {
    fn foobar() -> u32 { 23 }
    let f = concat_idents!(foo, bar);
    assert_eq!(f(), 23);
}