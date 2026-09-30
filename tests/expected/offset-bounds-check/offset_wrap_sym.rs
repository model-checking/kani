// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the offset UB check is not defeated by CBMC's pointer-offset
//! encoding, which wraps offsets at 2^(pointer_width - object_bits) rather
//! than 2^pointer_width: a symbolic byte offset that is a non-zero multiple
//! of 2^(64 - object_bits) (e.g. 2^48 with Kani's default of 16 object bits)
//! used to alias the original pointer, making the same-allocation check pass
//! spuriously and hiding genuine UB.
//! See <https://github.com/model-checking/kani/issues/1150>.
//!
//! CBMC fixed the underlying encoding wrap in
//! <https://github.com/diffblue/cbmc/pull/9134> (out-of-range results are
//! directed to the invalid object); these harnesses must produce the same
//! verdicts with and without that fix, via the address-arithmetic guard in
//! Kani's OffsetModel.

#[kani::proof]
fn check_ub_sym_wrap_positive() {
    let v = [0u8; 8];
    let p: *const u8 = &v[0];
    let off: isize = kani::any();
    kani::assume(off == 1isize << 48);
    let _q = unsafe { p.offset(off) }; // UB: far out of bounds
}

#[kani::proof]
fn check_ub_sym_wrap_negative() {
    let v = [0u8; 8];
    let p: *const u8 = &v[0];
    let off: isize = kani::any();
    kani::assume(off == -(1isize << 48));
    let _q = unsafe { p.offset(off) }; // UB: far out of bounds
}

#[kani::proof]
fn check_ub_sym_wrap_min() {
    let v = [0u8; 8];
    let p: *const u8 = &v[0];
    let off: isize = kani::any();
    kani::assume(off == isize::MIN);
    let _q = unsafe { p.byte_offset(off) }; // UB: far out of bounds
}

/// In-bounds negative offsets must still verify, including the extremes.
#[kani::proof]
fn check_valid_negative_offset() {
    let v = [0u8; 8];
    let p: *const u8 = &v[4];
    let off: isize = kani::any();
    kani::assume(off >= -4 && off <= 3);
    let q = unsafe { p.offset(off) };
    assert_eq!(unsafe { q.offset_from(v.as_ptr()) }, 4 + off);
}
