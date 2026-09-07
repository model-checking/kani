// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure that kani::any can generate valid std::ascii::EscapeDefault states.

use std::ascii::EscapeDefault;

#[kani::proof]
fn check_arbitrary_escape_default() {
    let escape: EscapeDefault = kani::any();
    let count = escape.count();

    assert!(count <= 4);

    kani::cover!(count == 0);
    kani::cover!(count == 1);
    kani::cover!(count == 2);
    kani::cover!(count == 3);
    kani::cover!(count == 4);
}
