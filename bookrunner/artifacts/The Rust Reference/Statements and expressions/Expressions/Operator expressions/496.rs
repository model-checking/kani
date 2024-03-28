// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct Addable;
use std::ops::AddAssign;

impl AddAssign<Addable> for Addable {
    /* */
fn add_assign(&mut self, other: Addable) {}
}

fn example() {
let (mut a1, a2) = (Addable, Addable);
  a1 += a2;

let (mut a1, a2) = (Addable, Addable);
  AddAssign::add_assign(&mut a1, a2);
}
}