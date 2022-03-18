// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#![feature(never_type)]
use std::mem::MaybeUninit;

// The code below attempts to instantiate uninhabited type `!`.
// This should cause the intrinsic `assert_inhabited` to generate a panic during
// compilation, but at present it triggers the `Nevers` hook instead.
// See https://github.com/model-checking/kani/issues/751
#[kani::proof]
fn main() {
    let _uninit_never: () = unsafe {
        MaybeUninit::<!>::uninit().assume_init();
    };
}
