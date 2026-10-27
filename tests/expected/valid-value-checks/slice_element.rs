// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Regression test for https://github.com/model-checking/kani/issues/5022:
//! reading one element of a validity-constrained type through a slice pointer
//! must be reported. A slice `[T]` has no static length, so the whole-slice
//! deref checks no elements; the accessed element is checked instead. A
//! symbolic index makes CBMC cover every element, so no length loop is needed.
//! `black_box` keeps each read from being removed as dead code.

/// Negative control: byte 2 read as a `bool` element is invalid.
#[kani::proof]
fn check_invalid_slice_elem_should_fail() {
    let bytes = [2u8; 4];
    let p = &bytes as *const [u8] as *const [bool];
    let _ = std::hint::black_box(unsafe { (*p)[1] });
}

/// Negative control, symbolic index: only index 2 holds an invalid `bool`
/// (byte 2); the rest are valid (byte 1). A symbolic index must still catch
/// the one bad element, which proves the check tracks the actual index rather
/// than a fixed position.
#[kani::proof]
fn check_invalid_slice_elem_symbolic_should_fail() {
    let bytes = [1u8, 1u8, 2u8, 1u8];
    let p = &bytes as *const [u8] as *const [bool];
    let i: usize = kani::any();
    kani::assume(i < 4);
    let _ = std::hint::black_box(unsafe { (*p)[i] });
}

/// Positive control: valid `bool` elements (byte 1) at any index verify.
#[kani::proof]
fn check_valid_slice_elem() {
    let bytes = [1u8; 4];
    let p = &bytes as *const [u8] as *const [bool];
    let i: usize = kani::any();
    kani::assume(i < 4);
    let _ = std::hint::black_box(unsafe { (*p)[i] });
}
