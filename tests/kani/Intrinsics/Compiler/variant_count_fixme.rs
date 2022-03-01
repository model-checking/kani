// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `variant_count` is supported and returns the expected result.

#![feature(variant_count)]
use std::mem;

enum Void {}
enum MyError {
    Error1,
    Error2,
    Error3,
}

fn main() {
    assert!(mem::variant_count::<Void>() == 0);
    assert!(mem::variant_count::<MyError>() == 3);
    assert!(mem::variant_count::<Option<u32>>() == 2);
    assert!(mem::variant_count::<Result<u32, MyError>>() == 2);
}
