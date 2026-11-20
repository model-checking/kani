// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn check_standalone() {
    let x: u8 = kani::any();
    assert_eq!(x.wrapping_add(1).wrapping_sub(1), x);
}
