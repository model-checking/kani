// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Exercise an actual projected Call destination, which ordinary source may lower to a temp.
#![feature(custom_mir, core_intrinsics)]

use std::intrinsics::mir::*;
use std::mem::MaybeUninit;

#[inline(never)]
fn produce_bool() -> bool {
    true
}

#[custom_mir(dialect = "runtime", phase = "optimized")]
unsafe fn call_into(ptr: *mut bool) {
    mir! {
        {
            Call(*ptr = produce_bool(), ReturnTo(done), UnwindUnreachable())
        }
        done = {
            Return()
        }
    }
}

#[kani::proof]
fn projected_call_destination() {
    let mut storage = MaybeUninit::<bool>::uninit();
    unsafe {
        call_into(storage.as_mut_ptr());
        assert!(storage.assume_init());
    }
    kani::cover!(true);
}
