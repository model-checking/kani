// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
'outer: loop {
    while true {
        break 'outer;
    }
}
}