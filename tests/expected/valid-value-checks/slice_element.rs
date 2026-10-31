// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Regression test for https://github.com/model-checking/kani/issues/5022:
//! reading an invalid element through a slice pointer must be reported (a slice
//! has no static length, so the whole-slice deref checks no elements). The check
//! fires only on a direct value read of the whole element, so forming a pointer
//! (`&raw const`), reading a sub-element, and writing are not reported. `Index`
//! and `ConstantIndex` share one arm, so the `Index` harnesses cover both.
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

/// Positive: forming a raw pointer to an element (`&raw const`) loads no `bool`,
/// so an element that would be invalid if read must not be reported.
#[kani::proof]
fn check_raw_ref_slice_elem_no_read() {
    let bytes = [2u8; 4];
    let p = &bytes as *const [u8] as *const [bool];
    let _ = std::hint::black_box(unsafe { &raw const (*p)[0] });
}

/// Positive: reading one `bool` of a `[bool; 2]` slice element checks only that
/// sub-byte. Byte 0 is a valid `bool`; the unread byte 1 is invalid but must not
/// cause a failure.
#[kani::proof]
fn check_slice_elem_partial_read() {
    let bytes = [[1u8, 2u8]];
    let p = &bytes as *const [[u8; 2]] as *const [[bool; 2]];
    let _ = std::hint::black_box(unsafe { (*p)[0][0] });
}

/// Positive: writing one `bool` of a `[bool; 2]` slice element must not validate
/// the element's old (invalid) contents, since a write is not UB until read.
#[kani::proof]
fn check_slice_elem_nested_write() {
    let mut bytes = [[2u8, 2u8]];
    let p = &mut bytes as *mut [[u8; 2]] as *mut [[bool; 2]];
    unsafe {
        (*p)[0][0] = true;
    }
}
