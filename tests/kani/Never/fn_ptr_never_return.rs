// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

//! Regression test for https://github.com/model-checking/kani/issues/4577
//!
//! Calling a function pointer whose return type is the never type `!` used to
//! panic the Kani compiler in `codegen_funcall`: the function-pointer path
//! unconditionally unwrapped the call's return target, but a diverging call
//! has no return target. Codegen must instead treat the missing target like a
//! direct call to a never-returning function does. Here the callee panics, so
//! verification reaches the panic and fails (rather than the compiler crashing).

fn diverge() -> ! {
    panic!("EXPECTED FAIL: diverging function was called");
}

#[kani::proof]
fn check_fnptr_never() {
    let f: fn() -> ! = diverge;
    f();
}
