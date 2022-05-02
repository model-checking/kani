// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn check_first_harness() {
    assert!(1 == 1);
}

#[kani::proof]
fn check_second_harness() {
    assert!(2 == 2);
}

fn main() {}
