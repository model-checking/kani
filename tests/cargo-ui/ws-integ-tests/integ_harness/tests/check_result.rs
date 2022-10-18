// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use integ_harness::*;

#[test]
fn test_no_overflow() {
    assert_eq!(rotate_right(0x0, 0), (0x0, false));
    assert_eq!(rotate_right(0x0, 100), (0x0, false));
    assert_eq!(rotate_right(0xF, 0), (0xF, false));
    assert_eq!(rotate_right(0x80, 7), (0x1, false));
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn no_panic() {
        rotate_right(kani::any(), kani::any());
    }
}
