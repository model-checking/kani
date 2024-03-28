// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
use std::cell::Cell;
let i : Cell<i32> = Cell::new(0);
match 1 {
    1 | _ if { i.set(i.get() + 1); false } => {}
    _ => {}
}
assert_eq!(i.get(), 2);
}