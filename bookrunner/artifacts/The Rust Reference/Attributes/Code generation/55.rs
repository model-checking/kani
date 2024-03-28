// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[cfg(target_feature = "avx2")]
#[target_feature(enable = "avx2")]
unsafe fn foo_avx2() {}
}