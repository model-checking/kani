// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --output-format terse
//! Keep the offset read in the call operand to test its codegen.

#![feature(core_intrinsics, custom_mir)]
#![allow(internal_features)]

use std::intrinsics::arith_offset;
use std::intrinsics::mir::*;

#[custom_mir(dialect = "runtime", phase = "optimized")]
unsafe fn offset_zst(base: *const (), offset: *const isize) -> *const () {
    mir! {
        {
            Call(RET = arith_offset(base, *offset), ReturnTo(done), UnwindUnreachable())
        }
        done = {
            Return()
        }
    }
}

#[kani::proof]
fn check_offset_operand() {
    let offset = kani::any::<usize>() as *const isize;
    kani::assume(offset.is_null());
    kani::cover!(true);
    let base = std::ptr::dangling::<()>();
    assert_eq!(unsafe { offset_zst(base, offset) }, base);
}
