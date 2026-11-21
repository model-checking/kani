// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Check that an invalid tag is reported for an enum whose layout is not a scalar or a scalar
//! pair, whose tag is one field of an aggregate
//! (<https://github.com/model-checking/kani/issues/5023>). It used to verify. Also check that
//! the tag of a directly tagged enum is one of its discriminants, not only in their range.

#[repr(C)]
struct Raw {
    a: u32,
    b: u32,
    c: u32,
}

/// Tag at byte 0, size 12.
#[allow(dead_code)]
#[repr(u8)]
enum G {
    A(u32, u32),
    B,
}

/// An enum in a struct, so that its tag is at a non-zero offset of the value read.
#[allow(dead_code)]
#[repr(C)]
struct Wrapper {
    pad: u32,
    g: G,
}

/// A `repr(C)` enum: a tag, then a union of the variants' fields.
#[allow(dead_code)]
#[repr(C)]
enum CEnum {
    X(u64),
    Y(u32, u32),
    Z,
}

/// Discriminants that are not contiguous: `2` to `4` are not valid tags.
#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(u8)]
enum Sparse {
    A = 1,
    B = 5,
}

#[kani::proof]
fn aggregate_enum_should_fail() {
    let raw = Raw { a: 7, b: 0, c: 0 };
    let g = unsafe { (&raw as *const Raw as *const G).read() }; // UB: tag 7
    std::mem::forget(g);
}

#[kani::proof]
fn aggregate_enum_valid() {
    let tag: u32 = kani::any_where(|tag| *tag < 2);
    let raw = Raw { a: tag, b: kani::any(), c: kani::any() };
    let g = unsafe { (&raw as *const Raw as *const G).read() };
    assert!(matches!(g, G::B) == (tag == 1));
}

#[kani::proof]
fn aggregate_enum_at_offset_should_fail() {
    let raw = [0u32, 2, 0, 0];
    let w = unsafe { (&raw as *const [u32; 4] as *const Wrapper).read() };
    std::mem::forget(w);
}

#[kani::proof]
fn aggregate_enum_at_offset_valid() {
    let raw = [kani::any(), 0u32, kani::any(), kani::any()];
    let w = unsafe { (&raw as *const [u32; 4] as *const Wrapper).read() };
    assert!(matches!(w.g, G::A(..)));
}

#[kani::proof]
fn repr_c_enum_any_tag_should_fail() {
    let raw: [u64; 2] = kani::any();
    let _e = unsafe { (&raw as *const [u64; 2] as *const CEnum).read() };
}

#[kani::proof]
fn repr_c_enum_valid() {
    let e = match kani::any::<u8>() {
        0 => CEnum::X(kani::any()),
        1 => CEnum::Y(kani::any(), kani::any()),
        _ => CEnum::Z,
    };
    let raw: [u64; 2] = unsafe { (&e as *const CEnum as *const [u64; 2]).read() };
    let back = unsafe { (&raw as *const [u64; 2] as *const CEnum).read() };
    assert!(std::mem::discriminant(&e) == std::mem::discriminant(&back));
}

#[kani::proof]
fn sparse_discriminant_should_fail() {
    let v: u8 = kani::any_where(|v| (2..5).contains(v));
    let _s: Sparse = unsafe { (&v as *const u8 as *const Sparse).read() };
}

#[kani::proof]
fn sparse_discriminant_valid() {
    let v: u8 = if kani::any() { 1 } else { 5 };
    let s: Sparse = unsafe { (&v as *const u8 as *const Sparse).read() };
    assert!(s as u8 == v);
}
