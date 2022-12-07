// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Different harnesses with the same name. This one should pass. The sibling crate should fail.

#[kani::proof]
fn check_lib() {
    assert!(1 == 1, "This should succeed");
}
