// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[forbid(missing_docs)]
pub mod m3 {
    // Attempting to toggle warning signals an error here
    #[allow(missing_docs)]
    /// Returns 2.
    pub fn undocumented_too() -> i32 { 2 }
}
}