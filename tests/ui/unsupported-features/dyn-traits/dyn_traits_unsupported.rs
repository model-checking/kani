// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that unsupported operations are caught correctly.
//! There are similar and more thorough test cases in the fixme regression.
//! Once the issues have been fixed, please delete this test in favor of the fixme tests.
#![allow(dead_code)]

use std::mem::{align_of_val, size_of_val};

trait T {}

struct A {
    id: u128,
}

impl T for A {}

/// From test fixme_size_of_fat_ptr.
/// Issue: https://github.com/model-checking/kani/issues/1074
#[kani::proof]
fn check_size_simple() {
    let a = A { id: 0 };
    let t: &dyn T = &a;
    assert_eq!(size_of_val(t), 16);
}

/// From test fixme_align_of_fat_ptr.
/// Issue: https://github.com/model-checking/kani/issues/1074
#[kani::proof]
fn check_align_simple() {
    let a = A { id: 0 };
    let t: &dyn T = &a;
    assert_eq!(align_of_val(t), 8);
}
