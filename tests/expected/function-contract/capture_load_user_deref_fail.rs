// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check that suppressing pointer-validity checks on contract-closure capture
//! loads does NOT suppress detection of an invalid *user-written* dereference
//! inside a contract clause: the clause below dereferences a dangling pointer
//! when evaluated at the call site, and verification must fail on the
//! `pointer_dereference` check inside the clause closure.

#[kani::requires(unsafe { *ptr } == 42)]
unsafe fn read_answer(ptr: *const i32) -> i32 {
    42
}

#[kani::proof]
fn harness() {
    let ptr = {
        let v = 42;
        &v as *const i32
    };
    // `v` is dead here, so evaluating the contract clause dereferences a
    // dangling pointer.
    let _ = unsafe { read_answer(ptr) };
}
