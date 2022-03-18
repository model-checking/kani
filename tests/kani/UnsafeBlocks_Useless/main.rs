// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    let x = unsafe {
        assert!(true);
        5
    };

    assert!(x == 5);
}
