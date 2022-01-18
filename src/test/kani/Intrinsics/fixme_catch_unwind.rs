// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// https://doc.rust-lang.org/std/panic/fn.catch_unwind.html
// Stable way of calling the `try` intrinsic.
use std::panic;

fn main() {
    let result = panic::catch_unwind(|| {
        println!("hello!");
    });
    assert!(result.is_ok());

    let result = panic::catch_unwind(|| {
        panic!("oh no!");
    });
    assert!(result.is_err());
}
