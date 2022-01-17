// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that rmc::any respect the char::MAX limit.
pub fn main() {
    let c: char = rmc::any();
    assert!(c <= char::MAX);
}
