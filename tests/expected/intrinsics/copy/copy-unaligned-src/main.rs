// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --legacy-linker
//! The MIR linker errors are not quite user friendly. For more details, see
//! <https://github.com/model-checking/kani/issues/1740>
//! Checks that `copy` fails when `src` is not aligned.
#[kani::proof]
fn test_copy_unaligned() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // Get an unaligned pointer with a single-byte offset
        let src_i8: *const i8 = src as *const i8;
        let src_unaligned = unsafe { src_i8.add(1) as *const i32 };
        let dst = src.add(1) as *mut i32;
        core::intrinsics::copy(src_unaligned, dst, 1);
    }
}
