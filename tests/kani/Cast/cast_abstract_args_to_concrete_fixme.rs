// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test is a modified version of cast_abstract_args_to_concrete_fixme.rs.
//! The original test requires --no-undefined-function-checks to pass. This is an issue that
//! require investigation. See https://github.com/model-checking/kani/issues/555.
//!
//! Once this issue is fixed. Please remove this test and remove the kani flag from the original
//! test: --no-undefined-function-check

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
