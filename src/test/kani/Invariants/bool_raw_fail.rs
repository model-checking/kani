// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that rmc::any_raw::<bool> may generate invalid booleans.

fn main() {
    let b: bool = unsafe { rmc::any_raw() };
    assert!(matches!(b, true | false), "Rust converts any non-zero value to true");
    match b {
        true => rmc::expect_fail(b as u8 == 1, "This can be any non-zero value"),
        false => assert!(b as u8 == 0),
    }
}
