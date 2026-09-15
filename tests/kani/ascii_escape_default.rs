// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Ensure that kani::any can generate valid std::ascii::EscapeDefault states.

use std::ascii::EscapeDefault;

#[kani::proof]
fn check_arbitrary_escape_default() {
    let mut escape: EscapeDefault = kani::any();

    let remaining = [escape.next(), escape.next(), escape.next(), escape.next()];

    // EscapeDefault yields at most four bytes.
    assert!(escape.next().is_none());

    let count = remaining.iter().filter(|byte| byte.is_some()).count();

    // Preserve the original remaining-length coverage.
    kani::cover!(count == 0);
    kani::cover!(count == 1);
    kani::cover!(count == 2);
    kani::cover!(count == 3);
    kani::cover!(count == 4);

    // Observable front-consumed state: b"\\x00" -> b"x00".
    kani::cover!(remaining == [Some(b'x'), Some(b'0'), Some(b'0'), None]);

    // Observable back-consumed state: b"\\x00" -> b"\\x".
    kani::cover!(remaining == [Some(b'\\'), Some(b'x'), None, None]);

    // Observable mixed front/back state: b"\\x00" -> b"x0".
    kani::cover!(remaining == [Some(b'x'), Some(b'0'), None, None]);
}
