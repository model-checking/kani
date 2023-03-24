// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that verfication fails when `#[kani::should_panic]` is used and no
//! panics are encountered.

#[kani::proof]
#[kani::should_panic]
fn check() {
    assert!(1 + 1 == 2);
}
