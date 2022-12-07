// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Different harnesses with the same name. This one should fail. The sibling crate should pass.

#[kani::proof]
fn check_lib() {
    assert_eq!(1 + 1, 1, "This should fail");
}
