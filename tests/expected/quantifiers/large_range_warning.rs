// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zquantifiers --solver z3

//! Test that Kani emits a compile-time warning when a quantifier has a large
//! statically-known range. Unbounded quantifiers (no explicit bounds) expand to
//! usize::MIN..usize::MAX, which are visible as constants at codegen time.
//! Bounded quantifiers use let bindings for type coercion, so their constants
//! are not visible at codegen time — a known limitation.
//!
//! Note: We use --solver z3 so the harnesses actually verify (SAT would hang
//! on unbounded ranges). The warning fires at codegen time regardless of solver.

#[kani::proof]
fn check_unbounded_forall_warns() {
    // Unbounded quantifier expands to usize::MIN..usize::MAX: should warn
    assert!(kani::forall!(|i| i < 10 || i >= 10));
}

#[kani::proof]
fn check_unbounded_exists_warns() {
    // Unbounded exists also warns
    assert!(kani::exists!(|i| i == 42));
}

#[kani::proof]
fn check_small_forall_no_warn() {
    // Range of 10 < threshold: should NOT trigger warning.
    // (No expected output for this harness — absence of warning is the test.)
    assert!(kani::forall!(|i in (0, 10)| i < 10));
}
