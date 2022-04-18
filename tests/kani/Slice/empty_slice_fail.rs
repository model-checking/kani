// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-verify-fail

//! Empty slices use dangling pointers. We disabled overflow check to avoid a bunch of spurious
//! counter examples. This test checks that we still fail on improper dereference check.

#[kani::proof]
fn check_invalid_access_fails() {
    let vector = Vec::<bool>::new();
    let slice = vector.as_slice();
    let ptr = slice.as_ptr();
    let _first = unsafe { *ptr };
}
