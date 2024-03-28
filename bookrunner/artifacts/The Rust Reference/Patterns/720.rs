// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let int_reference = &3;
match int_reference {
    &(0..=5) => (),
    _ => (),
}
}