// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: --edition 2021 -C panic=abort -D warnings

#![feature(core_intrinsics, custom_mir)]
#![allow(internal_features)]

use core::intrinsics::mir::*;

union Word {
    word: u32,
}
fn identity(value: u32) -> u32 {
    value
}

// No statements in bb0: LoadArgument and the union-field Check share the
// unresolved Call's target. The initialized control requires the load to survive.
#[custom_mir(dialect = "runtime", phase = "optimized")]
#[inline(never)]
unsafe fn indirect(value: Word, f: fn(u32) -> u32) -> u32 {
    mir! {
        { Call(RET = f(value.word), ReturnTo(done), UnwindUnreachable()) }
        done = { Return() }
    }
}

#[kani::proof]
fn check() {
    let _ = unsafe { indirect(Word { word: 42 }, identity) };
}
