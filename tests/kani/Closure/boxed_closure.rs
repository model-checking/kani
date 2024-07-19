// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: -Zmir-opt-level=2

fn call_boxed_closure(f: Box<dyn Fn() -> ()>) -> () {
    f()
}

#[kani::proof]
fn main() {
    let x = 1;
    let closure = move || {let _ = x; ()};
    call_boxed_closure(Box::new(closure));
}
