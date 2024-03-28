// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
struct HoldsCallable<F: Fn()> { callable: F }
let holds_callable = HoldsCallable { callable: || () };

// Invalid: Parsed as calling the method "callable"
// holds_callable.callable();

// Valid
(holds_callable.callable)();
}