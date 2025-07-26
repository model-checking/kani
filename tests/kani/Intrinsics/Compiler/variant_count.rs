// Copyright Kani Contributors
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

#[kani::proof]
fn main() {
    const VOID_COUNT: usize = mem::variant_count::<Void>();
    const ERROR_COUNT: usize = mem::variant_count::<MyError>();
    const OPTION_COUNT: usize = mem::variant_count::<Option<u32>>();
    const RESULT_COUNT: usize = mem::variant_count::<Result<u32, MyError>>();

    assert!(VOID_COUNT == 0);
    assert!(ERROR_COUNT == 3);
    assert!(OPTION_COUNT == 2);
    assert!(RESULT_COUNT == 2);
}
