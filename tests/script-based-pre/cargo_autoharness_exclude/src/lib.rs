// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Test that the automatic harness generation feature selects functions correctly
// when --exclude-function is provided.

mod include {
    fn simple(x: u8, _y: u16) -> u8 {
        x
    }

    // Doesn't implement Arbitrary, so still should not be included.
    fn generic<T>(x: u32, _y: T) -> u32 {
        x
    }
}

// These functions are eligible for autoverification, but are excluded.
mod excluded {
    fn simple(x: u8, _y: u16) -> u8 {
        x
    }
}
