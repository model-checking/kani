// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Ensure the panic!() internal to `Option::unwrap()` is stubbed.
#[kani::proof]
fn main() {
    foo();
}

fn foo() -> usize {
    let a: Option<usize> = kani::any();
    a.unwrap()
}
