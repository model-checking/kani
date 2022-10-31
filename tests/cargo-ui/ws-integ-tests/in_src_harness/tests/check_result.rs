// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use in_src_harness::*;

#[test]
fn test_no_overflow() {
    assert_eq!(rotate_left(0x0, 0), (0x0, false));
    assert_eq!(rotate_left(0x0, 100), (0x0, false));
    assert_eq!(rotate_left(0xF, 0), (0xF, false));
    assert_eq!(rotate_left(0x1, 7), (0x80, false));
}
