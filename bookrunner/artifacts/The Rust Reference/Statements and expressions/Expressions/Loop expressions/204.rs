// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let mut last = 0;
for x in 1..100 {
    if x > 12 {
        break;
    }
    last = x;
}
assert_eq!(last, 12);
}