// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
use std::sync::atomic::Ordering;
use std::sync::atomic;
atomic::fence(Ordering::Acquire);
}