// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
use std::cmp::Ordering;

unsafe trait UnsafeOrd {
    fn cmp(&self, other: &Self) -> Ordering;
}
}