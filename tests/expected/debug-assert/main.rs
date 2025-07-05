// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that execution continues beyond a failing debug_assert (but does not in case of
// a failing assert).

#[kani::proof]
fn foo() {
    std::debug_assert!(false, "will fail");
    std::assert!(false, "will fail");
    std::debug_assert!(false, "not reached");
}
