// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The second assertion fails, but the fake `cbmc` drops it from the result array.

#[kani::proof]
fn check() {
    let x: u8 = kani::any();
    kani::assume(x < 20);
    assert!(x < 20);
    assert!(x < 10);
}
