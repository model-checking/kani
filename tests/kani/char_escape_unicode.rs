// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Ensure that `kani::any` can generate valid `std::char::EscapeUnicode` states.

use std::char::EscapeUnicode;

#[kani::proof]
fn check_arbitrary_escape_unicode() {
    let escape: EscapeUnicode = kani::any();
    let count = escape.count();

    assert!(count <= 10);

    kani::cover!(count == 0);
    kani::cover!(count == 1);
    kani::cover!(count == 2);
    kani::cover!(count == 3);
    kani::cover!(count == 4);
    kani::cover!(count == 5);
    kani::cover!(count == 6);
    kani::cover!(count == 7);
    kani::cover!(count == 8);
    kani::cover!(count == 9);
    kani::cover!(count == 10);
}
