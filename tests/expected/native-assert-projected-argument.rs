// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z unstable-options --no-assert-overrides --output-format terse
//! Keep the diagnostic read in the call operand to test the panic hook.

#![feature(core_intrinsics, custom_mir, panic_internals)]
#![allow(internal_features)]

extern crate core;

use core::fmt::Arguments;
use core::panicking::{AssertKind, assert_failed};
use std::intrinsics::mir::*;

#[custom_mir(dialect = "runtime", phase = "optimized")]
unsafe fn fail_assertion(
    kind: AssertKind,
    left: &u8,
    right: &u8,
    message: *const Option<Arguments<'static>>,
) -> ! {
    mir! {
        {
            Call(RET = assert_failed(kind, left, right, *message), ReturnTo(done), UnwindUnreachable())
        }
        done = {
            Unreachable()
        }
    }
}

#[kani::proof]
#[kani::should_panic]
fn check_projected_diagnostic_operand() {
    let message = kani::any::<usize>() as *const Option<Arguments<'static>>;
    kani::assume(message.is_null());
    kani::cover!(true);
    unsafe { fail_assertion(AssertKind::Eq, &0u8, &1u8, message) };
}
