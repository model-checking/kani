// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Ensure that the panic!() macro itself gets stubbed.
#[kani::proof]
fn main() {
    panic!("hello!");
}
