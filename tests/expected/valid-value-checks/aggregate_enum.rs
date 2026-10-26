// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Regression test for https://github.com/model-checking/kani/issues/5023:
//! an invalid discriminant of a directly-tagged enum whose layout is an
//! aggregate (not a scalar or scalar pair) must be reported. The tag was
//! previously unchecked because the validity requirement was derived only from
//! the enum's top-level scalar, which an aggregate layout does not have.

#[allow(dead_code)]
#[repr(C)]
struct Raw {
    a: u32,
    b: u32,
    c: u32,
}

// `#[repr(u8)]` with a multi-field payload forces an aggregate layout: a `u8`
// tag at offset 0, then two `u32` payload fields at offsets 4 and 8. The valid
// tags are 0 (`A`) and 1 (`B`); the tag byte is byte 0 of `Raw` (the low byte
// of `a` on a little-endian target).
#[allow(dead_code)]
#[repr(u8)]
enum G {
    A(u32, u32),
    B,
}

/// Negative control: tag byte 7 is not a valid discriminant of `G`.
#[kani::proof]
fn check_invalid_aggregate_enum_tag_should_fail() {
    let raw = Raw { a: 7, b: 0, c: 0 };
    // `forget` avoids dropping an enum with an invalid discriminant, which
    // would be separate UB and could mask the value-validity check.
    let g = unsafe { (&raw as *const Raw as *const G).read() };
    std::mem::forget(g);
}

/// Negative control at the boundary: tag byte 2 is the first value above the
/// valid range `0..=1`; a too-wide range would wrongly accept it.
#[kani::proof]
fn check_boundary_invalid_aggregate_enum_tag_should_fail() {
    let raw = Raw { a: 2, b: 0, c: 0 };
    let g = unsafe { (&raw as *const Raw as *const G).read() };
    std::mem::forget(g);
}

/// Positive control: every valid tag (0 and 1) reads without a spurious failure.
#[kani::proof]
fn check_valid_aggregate_enum_tag() {
    let tag: u32 = kani::any();
    kani::assume(tag <= 1);
    let raw = Raw { a: tag, b: 0, c: 0 };
    let g = unsafe { (&raw as *const Raw as *const G).read() };
    std::mem::forget(g);
}
