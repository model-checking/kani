// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z c-ffi --c-lib tests/expected/c-ffi-void/void_fns.c

//! Check that a C function returning `void` is really executed, both when called directly and
//! through a Rust function pointer: an assertion its side effect contradicts must fail, and a
//! cover of the effect must be satisfied. (A dropped call, or a pointer call that resolved to no
//! target, would let the assertion pass.) The same holds for C functions returning `void` that
//! Rust declares as not returning (`-> !`); if one returns anyway, Kani reports it.

extern "C" {
    /// `void decrement(uint32_t *p)` in C.
    fn decrement(p: &mut u32);
    /// `void add_one(uint32_t *p)` in C.
    fn add_one(p: &mut u32);
    /// `void double_it(uint32_t *p)` in C.
    fn double_it(p: &mut u32);
    /// `void halt(void)` in C, which does not return.
    fn halt() -> !;
    /// `void returns_anyway(void)` in C, which returns.
    fn returns_anyway() -> !;
}

#[kani::proof]
fn check_direct_call() {
    let mut x: u32 = 1;
    unsafe { decrement(&mut x) };
    kani::cover!(x == 0, "direct call decremented");
    assert!(x == 1, "direct call left x unchanged");
}

#[kani::proof]
fn check_fn_ptr_call() {
    let mut x: u32 = 1;
    let f: unsafe extern "C" fn(&mut u32) = decrement;
    unsafe { f(&mut x) };
    kani::cover!(x == 0, "pointer call decremented");
    assert!(x == 1, "pointer call left x unchanged");
}

/// A function pointer that may point to either of two `void` functions. Each target must stay a
/// candidate of the pointer call: one that was dropped would leave its cover unsatisfiable.
#[kani::proof]
fn check_fn_ptr_two_targets() {
    let before: u32 = kani::any_where(|v| *v < 1000);
    let use_add: bool = kani::any();
    let f: unsafe extern "C" fn(&mut u32) = if use_add { add_one } else { double_it };
    let mut x = before;
    unsafe { f(&mut x) };
    kani::cover!(use_add && x == before + 1, "called add_one through the pointer");
    kani::cover!(!use_add && x == before * 2, "called double_it through the pointer");
    assert!(x == if use_add { before + 1 } else { before * 2 }, "pointer call had its effect");
}

#[kani::proof]
fn check_never_call() {
    let stop: bool = kani::any();
    kani::cover!(stop, "calling halt");
    if stop {
        unsafe { halt() }
    }
    assert!(!stop, "halt returned");
}

#[kani::proof]
fn check_never_returns_anyway() {
    unsafe { returns_anyway() }
}
