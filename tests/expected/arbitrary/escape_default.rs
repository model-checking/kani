// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure that kani::any can generate valid std::ascii::EscapeDefault states,
//! including states consumed from the front, from the back, and from both ends.

use std::ascii::EscapeDefault;

#[kani::proof]
fn check_arbitrary_escape_default() {
    let mut escape: EscapeDefault = kani::any();

    let remaining = [escape.next(), escape.next(), escape.next(), escape.next()];

    // EscapeDefault yields at most four bytes.
    assert!(escape.next().is_none());

    let count = remaining.iter().filter(|byte| byte.is_some()).count();

    // Every remaining length from fully consumed to fully unconsumed is reachable.
    kani::cover!(count == 0, "0 bytes remaining");
    kani::cover!(count == 1, "1 byte remaining");
    kani::cover!(count == 2, "2 bytes remaining");
    kani::cover!(count == 3, "3 bytes remaining");
    kani::cover!(count == 4, "4 bytes remaining");

    // Observable front-consumed state: b"\\x00" -> b"x00".
    kani::cover!(remaining == [Some(b'x'), Some(b'0'), Some(b'0'), None], "front consumed");

    // Observable back-consumed state: b"\\x00" -> b"\\x".
    kani::cover!(remaining == [Some(b'\\'), Some(b'x'), None, None], "back consumed");

    // Observable mixed front/back state: b"\\x00" -> b"x0".
    kani::cover!(remaining == [Some(b'x'), Some(b'0'), None, None], "front and back consumed");
}
