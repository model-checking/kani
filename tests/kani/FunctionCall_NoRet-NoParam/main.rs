// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    assert!(true);
    trivial_function();
    assert!(1.0 == 1.0);
}
fn trivial_function() {
    assert!(1 + 1 == 2);
}
