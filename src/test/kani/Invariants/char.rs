// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that kani::any respect the char::MAX limit.
pub fn main() {
    let c: char = kani::any();
    assert!(c <= char::MAX);
}
