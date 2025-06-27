// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Test that Kani can properly handle closure to fn ptr when some of the arguments are zero
//! size type.
//! Also ensure that we can still take the address of the arguments.

struct Void;

/// Invoke given function with the given 'input'.
fn invoke(input: usize, f: fn(Void, usize, Void) -> usize) -> usize {
    kani::cover!();
    f(Void, input, Void)
}

#[kani::proof]
fn check_zst_param() {
    let input = kani::any();
    let closure = |a: Void, out: usize, b: Void| {
        kani::cover!();
        assert!(&a as *const Void != std::ptr::null(), "Should succeed");
        assert!(&b as *const Void != std::ptr::null(), "Should succeed");
        out
    };
    let output = invoke(input, closure);
    assert_eq!(output, input);
}
