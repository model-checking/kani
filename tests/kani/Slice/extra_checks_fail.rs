// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --extra-pointer-checks
// kani-verify-fail

//! Empty slices use dangling pointers. With extra pointer checks, this test fails due to
//! arithmetic operations using a dangling pointer

#[kani::proof]
fn check_empty_fails() {
    let vec = Vec::<f32>::new();
    for float in vec {
        assert!(float.is_nan());
    }
}
