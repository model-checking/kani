// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: --edition 2021 -C panic=abort -D warnings

#![feature(core_intrinsics, custom_mir)]
#![allow(internal_features)]

use core::intrinsics::mir::*;

fn identity(value: u8) -> u8 {
    value
}

// Check the dereferenced call argument before reporting the unsupported function-pointer call.
#[custom_mir(dialect = "runtime", phase = "optimized")]
#[inline(never)]
unsafe fn indirect(f: fn(u8) -> u8, pointer: *const u8) -> u8 {
    mir! {
        { Call(RET = f(*pointer), ReturnTo(done), UnwindUnreachable()) }
        done = { Return() }
    }
}

#[kani::proof]
fn check() {
    // Allocate two bytes and initialize the first byte.
    let layout = std::alloc::Layout::new::<[u8; 2]>();
    let pointer = unsafe { std::alloc::alloc(layout) };
    if pointer.is_null() {
        return;
    }
    unsafe {
        *pointer = 7;
        // The second byte remains uninitialized until the Call operand reads it.
        let _ = indirect(identity, pointer.add(1));
        std::alloc::dealloc(pointer, layout);
    }
}
