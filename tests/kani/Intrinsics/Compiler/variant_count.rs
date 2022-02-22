// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `variant_count` is not supported.
// Commented out code below can be enabled to ensure the implementation works as
// expected when support is added
#![feature(variant_count)]
use std::mem;

enum Void {}
enum MyError {
    Error1,
    Error2,
}

fn main() {
    let _ = mem::variant_count::<Void>();
    // assert!(mem::variant_count::<Void>() == 0);
    // assert!(mem::variant_count::<MyError>() == 2);
    // assert!(mem::variant_count::<Option<u32>>() == 2);
    // assert!(mem::variant_count::<Result<u32, MyError>>() == 2);
}
