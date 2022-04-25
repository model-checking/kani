// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    let a: u32 = kani::any();
    assert!(a / 2 <= a);
    assert!(a / 2 * 2 >= a / 2);
    assert!(a / 2 + 5 + 1 > a / 2 + 5);
    assert!(a / 2 + 5 + 1 - 2 < a / 2 + 5);
}
