// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check that a *user-written* dereference inside a contract clause still gets
//! pointer-validity checks, even though the capture loads in the same closure
//! do not (see capture_load_checks_elided.rs): only the load of the captured
//! `ptr` value itself is exempt from checking; dereferencing what it points to
//! is a separate operation on a different MIR local and remains fully checked.
//! The expected file requires `pointer_dereference` checks to be present and
//! passing inside the clause closure.
//!
//! The total is plumbing-sensitive: nightly-2026-06-01 makes two checks in `read_answer`'s own body
//! unreachable, so the count drops by two. The pinned closure check -- the actual subject here --
//! is unaffected.

#[kani::requires(unsafe { *ptr } == 42)]
unsafe fn read_answer(ptr: *const i32) -> i32 {
    unsafe { *ptr }
}

#[kani::proof]
fn harness() {
    let v = 42;
    let _ = unsafe { read_answer(&v) };
}
