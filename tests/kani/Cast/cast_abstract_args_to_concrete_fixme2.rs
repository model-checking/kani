// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// https://github.com/model-checking/kani/issues/555
// kani-flags: --no-undefined-function-checks

// This regression test is in response to issue #135.
// The type of the second parameter to powi is a `CInteger`, but
// the type of `2` here is a `u32`. This test ensures that
// kani automatically casts the `2` to a `CInteger`.

// More generally, this acts as a stand-in to make sure that
// abstract types (e.g. u32, i32) are automatically casted to
// their corresponding concrete types (e.g. int, float) when necessary.

// The only function with this issue I could find was `powi`
// for `f32` and `f64`. The only other built-in that uses a
// `c_int` type is `libc::memset`, which seems to work fine,
// but is included here for certainty.
#![feature(rustc_private)]
extern crate libc;

use std::mem;

#[kani::proof]
fn main() {
    let _x32 = 1.0f32.powi(2);
    let _x64 = 1.0f64.powi(2);

    unsafe {
        let size: libc::size_t = mem::size_of::<i32>();
        let my_num: *mut libc::c_void = libc::malloc(size);
        if my_num.is_null() {
            panic!("failed to allocate memory");
        }
        let my_num2 = libc::memset(my_num, 1, size);
        libc::free(my_num);
    }
}
