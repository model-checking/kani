// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z c-ffi --c-lib tests/expected/c-ffi-void/void_fns.c

//! Check that a C function returning `void` is really executed, both when called directly and
//! through a Rust function pointer: an assertion its side effect contradicts must fail, and a
//! cover of the effect must be satisfied. (A dropped call, or a pointer call that resolved to no
//! target, would let the assertion pass.)

extern "C" {
    /// `void decrement(uint32_t *p)` in C.
    fn decrement(p: &mut u32);
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
