// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --output-format old

// kani-flags: --function check_assert
// compile-flags: --crate-type lib
#[no_mangle]
pub fn check_assert() {
    let x: u8 = kani::any();
    let y = x;
    assert!(x == y);
}
