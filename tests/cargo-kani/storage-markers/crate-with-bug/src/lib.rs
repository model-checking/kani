// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This function contains a use-after-free bug.

pub fn fn_with_bug() -> i32 {
    let raw_ptr = {
        let var = 10;
        &var as *const i32
    };
    unsafe { *raw_ptr }
}
