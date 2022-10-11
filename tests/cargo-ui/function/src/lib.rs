// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This is just to test that cargo kani --debug works.

#[no_mangle]
pub fn harness() {
    assert_eq!(1 + 2, 3);
}
