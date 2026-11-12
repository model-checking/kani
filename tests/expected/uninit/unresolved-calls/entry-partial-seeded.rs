// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: --edition 2021 -C panic=abort -D warnings

#![feature(core_intrinsics, custom_mir)]
#![allow(internal_features)]

use core::intrinsics::mir::*;

union Word {
    byte: u8,
    word: u32,
}
fn identity(value: u32) -> u32 {
    value
}

// No statements in bb0: preserve the entry LoadArgument and the Check
// collected from the union field used as the call argument.
#[custom_mir(dialect = "runtime", phase = "optimized")]
#[inline(never)]
unsafe fn indirect(value: Word, f: fn(u32) -> u32) -> u32 {
    mir! {
        { Call(RET = f(value.word), ReturnTo(done), UnwindUnreachable()) }
        done = { Return() }
    }
}

// A copy intrinsic creates a delayed-UB analysis target without introducing UB.
#[inline(never)]
fn seed() {
    let source = 7u8;
    let mut destination = 0u8;
    unsafe { std::ptr::copy_nonoverlapping(&source, &mut destination, 1) };
    assert_eq!(destination, source);
}

#[kani::proof]
fn check() {
    seed();
    let _ = unsafe { indirect(Word { byte: 42 }, identity) };
}
