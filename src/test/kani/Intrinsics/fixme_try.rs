// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// See discussion on https://github.com/model-checking/rmc/issues/267
#![feature(core_intrinsics)]
use std::intrinsics::r#try;

fn main() {
    unsafe {
        // Rust will make a best-effort to swallow the panic, and then execute the cleanup function.
        // However, my understanding is that failure is still possible, since its just a best-effort
        r#try(
            |_a: *mut u8| panic!("foo"),
            std::ptr::null_mut(),
            |_a: *mut u8, _b: *mut u8| println!("bar"),
        );
    }
}
