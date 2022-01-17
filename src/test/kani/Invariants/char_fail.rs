// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that rmc::any_raw may generate invalid char.
pub fn main() {
    let c: char = unsafe { rmc::any_raw() };
    rmc::expect_fail(c <= char::MAX, "rmc::any_raw() may generate invalid values");
}
