// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: -Zmir-opt-level=2

// The main function of this test moves an integer into a closure,
// boxes the value, then passes the closure to a function that calls it.
// This test covers the issue
// https://github.com/model-checking/kani/issues/2874 .

fn call_boxed_closure(f: Box<dyn Fn() -> ()>) -> () {
    f()
}

// #[kani::proof]
fn main() {
    let x = 1;
    let closure = move || {
        let _ = x;
        ()
    };
    call_boxed_closure(Box::new(closure));
}
