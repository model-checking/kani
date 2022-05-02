// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    assert!(1 == 2);
}

#[kani::proof]
fn harness() {
    assert!(3 == 4);
}
