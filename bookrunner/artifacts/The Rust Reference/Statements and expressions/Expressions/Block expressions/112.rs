// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
loop {
    async move {
        break; // This would break out of the loop.
    }
}
}