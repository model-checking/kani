// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Empty slices use dangling pointers. Ensure that Kani is ok with that.

#[kani::proof]
fn check_empty() {
    let vec = Vec::<f32>::new();
    for float in vec {
        assert!(float.is_nan());
    }
}
