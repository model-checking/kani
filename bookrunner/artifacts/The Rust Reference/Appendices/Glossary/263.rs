// compile-flags: --edition 2021
// kani-flags: --enable-unstable --cbmc-args --unwind 4
#![allow(unused)]
fn main() {
let ok_num = Ok::<_, ()>(5);
assert!(!ok_num.is_err());
let vec = [1, 2, 3].iter().map(|n| n * 2).collect::<Vec<_>>();
assert!([2, 4, 6][..] == vec[..]);
}