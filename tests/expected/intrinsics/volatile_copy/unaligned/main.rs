// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `volatile_copy_memory` fails when `dst` is not aligned.
//
// This specifically misaligns `dst` (and not `src`) because
// `volatile_copy_memory` is declared as `(dst, src, count)`, the reverse of
// the `(src, dst, ..)` order that `codegen_copy` expects internally, so
// codegen must swap `fargs`/`farg_types` before delegating to it. Misaligning
// `dst` while keeping `src` aligned distinguishes a correct swap (which
// reports "`dst` must be properly aligned") from a missing/backwards swap
// (which would instead misreport "`src` must be properly aligned").
#![feature(core_intrinsics)]

#[kani::proof]
fn test_volatile_copy_memory_unaligned_dst() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // Obtain an unaligned pointer by casting into `*const i8`, adding an
        // offset of 1 and casting back into `*mut i32`.
        let dst_i8: *const i8 = src as *const i8;
        let dst_unaligned = dst_i8.add(1) as *mut i32;
        core::intrinsics::volatile_copy_memory(dst_unaligned, src, 1);
    }
}
