// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Check the validity of enums with more than one variant whose fields have validity
//! requirements (<https://github.com/model-checking/kani/issues/3761>): the tag must encode the
//! discriminant of an inhabited variant, and the fields of the variant it selects must be valid.
//! These used to be reported as unsupported.
//!
//! The values are built by typed reads of bytes written untyped (into a valid value, or as an
//! integer), as `transmute` to an enum also gets rustc's own discriminant check.

use std::mem::size_of;

/// A directly tagged enum.
#[derive(Clone, Copy, kani::Arbitrary)]
#[allow(dead_code)]
enum Tagged {
    A(bool),
    B(char),
    C(u8),
}

/// A niche-encoded enum: `B` and `C` are encoded by values after `char::MAX` in the field of `A`.
#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Niche {
    A(char),
    B,
    C,
}

/// A niche-encoded enum whose niche is one field of a payload with two fields.
#[derive(Clone, Copy)]
#[allow(dead_code)]
enum NichePair {
    A(u8, bool),
    B,
}

/// Nested enums, each with field requirements.
#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Outer {
    X(Tagged),
    Y(Option<char>),
    Z(bool, Niche),
}

#[allow(dead_code)]
enum Void {}

/// The variant `B` is uninhabited, but it is not absent from the layout.
#[allow(dead_code)]
enum WithUninhabited {
    A(u16),
    B(u16, Void),
}

/// The byte at `idx` of `value`.
fn byte_of<T>(value: &T, idx: usize) -> u8 {
    unsafe { (value as *const T as *const u8).add(idx).read() }
}

/// The index of the first byte that differs between `a` and `b`.
fn first_diff<T>(a: &T, b: &T) -> usize {
    (0..size_of::<T>()).find(|idx| byte_of(a, *idx) != byte_of(b, *idx)).unwrap()
}

/// `value` with the byte at `idx` replaced by `byte`, read back as a `T`.
fn with_byte<T>(mut value: T, idx: usize, byte: u8) -> T {
    unsafe {
        (&mut value as *mut T as *mut u8).add(idx).write(byte);
        (&value as *const T).read()
    }
}

#[kani::proof]
fn tagged_any_bytes_should_fail() {
    let bytes: u64 = kani::any();
    let _e: Tagged = unsafe { (&bytes as *const u64 as *const Tagged).read() };
}

#[kani::proof]
fn tagged_valid() {
    let e: Tagged = kani::any();
    let mut bytes = 0u64;
    unsafe { (&mut bytes as *mut u64 as *mut Tagged).write(e) };
    let back: Tagged = unsafe { (&bytes as *const u64 as *const Tagged).read() };
    match (e, back) {
        (Tagged::A(x), Tagged::A(y)) => assert!(x == y),
        (Tagged::B(x), Tagged::B(y)) => assert!(x == y),
        (Tagged::C(x), Tagged::C(y)) => assert!(x == y),
        _ => unreachable!(),
    }
}

#[kani::proof]
#[kani::unwind(9)]
fn tagged_invalid_bool_field_should_fail() {
    let idx = first_diff(&Tagged::A(true), &Tagged::A(false));
    let _e = with_byte(Tagged::A(true), idx, 2);
}

#[kani::proof]
#[kani::unwind(9)]
fn tagged_unrestricted_field() {
    // The field of `C` has no requirement, even where the field of `A` has one.
    let idx = first_diff(&Tagged::A(true), &Tagged::A(false));
    let e = with_byte(Tagged::C(0), idx, kani::any());
    assert!(matches!(e, Tagged::C(_)));
}

#[kani::proof]
#[kani::unwind(9)]
fn tagged_invalid_tag_should_fail() {
    let idx = first_diff(&Tagged::A(true), &Tagged::C(1));
    let _e = with_byte(Tagged::A(true), idx, 3);
}

#[kani::proof]
fn tagged_surrogate_should_fail() {
    let mut e = Tagged::B('a');
    if let Tagged::B(c) = &mut e {
        unsafe { (c as *mut char as *mut u32).write(0xD800) };
    }
    let _e = unsafe { (&e as *const Tagged).read() };
}

#[kani::proof]
fn niche_valid() {
    let n = match kani::any::<u8>() {
        0 => Niche::A(kani::any()),
        1 => Niche::B,
        _ => Niche::C,
    };
    let v: u32 = unsafe { (&n as *const Niche as *const u32).read() };
    let back: Niche = unsafe { (&v as *const u32 as *const Niche).read() };
    match (n, back) {
        (Niche::A(a), Niche::A(b)) => assert!(a == b && a as u32 == v),
        (Niche::B, Niche::B) | (Niche::C, Niche::C) => assert!(char::from_u32(v).is_none()),
        _ => unreachable!(),
    }
}

#[kani::proof]
fn niche_surrogate_should_fail() {
    let v = 0xDFFFu32;
    let _n: Niche = unsafe { (&v as *const u32 as *const Niche).read() };
}

#[kani::proof]
fn niche_any_should_fail() {
    // Two of the values that aren't `char`s encode `B` and `C`, the others are invalid.
    let v: u32 = kani::any();
    kani::assume(char::from_u32(v).is_none());
    let _n: Niche = unsafe { (&v as *const u32 as *const Niche).read() };
}

#[kani::proof]
fn option_char_valid() {
    let o: Option<char> = if kani::any() { Some(kani::any()) } else { None };
    let v: u32 = unsafe { (&o as *const Option<char> as *const u32).read() };
    let back: Option<char> = unsafe { (&v as *const u32 as *const Option<char>).read() };
    assert!(o == back);
}

#[kani::proof]
fn option_char_any_should_fail() {
    let v: u32 = kani::any();
    let _o: Option<char> = unsafe { (&v as *const u32 as *const Option<char>).read() };
}

#[kani::proof]
fn niche_pair_valid() {
    let n = if kani::any() { NichePair::A(kani::any(), kani::any()) } else { NichePair::B };
    let v: u16 = unsafe { (&n as *const NichePair as *const u16).read() };
    let back: NichePair = unsafe { (&v as *const u16 as *const NichePair).read() };
    assert!(matches!(n, NichePair::B) == matches!(back, NichePair::B));
}

#[kani::proof]
fn niche_pair_any_should_fail() {
    let bytes: u16 = kani::any();
    let _n: NichePair = unsafe { (&bytes as *const u16 as *const NichePair).read() };
}

#[kani::proof]
fn nested_valid() {
    let o = match kani::any::<u8>() {
        0 => Outer::X(kani::any()),
        1 => Outer::Y(if kani::any() { Some(kani::any()) } else { None }),
        _ => Outer::Z(kani::any(), if kani::any() { Niche::A(kani::any()) } else { Niche::C }),
    };
    let copy = o;
    let back = unsafe { (&copy as *const Outer).read() };
    assert!(matches!(
        (o, back),
        (Outer::X(_), Outer::X(_)) | (Outer::Y(_), Outer::Y(_)) | (Outer::Z(..), Outer::Z(..))
    ));
}

#[kani::proof]
fn nested_inner_invalid_should_fail() {
    // A valid `Outer::Y` whose `char` is replaced by a surrogate.
    let mut o = Outer::Y(Some('a'));
    if let Outer::Y(Some(c)) = &mut o {
        unsafe { (c as *mut char as *mut u32).write(0xD800) };
    }
    let _o = unsafe { (&o as *const Outer).read() };
}

#[kani::proof]
fn nested_inner_tag_invalid_should_fail() {
    // A valid `Outer::X` whose inner `Tagged::A` has an invalid `bool`.
    let mut o = Outer::X(Tagged::A(false));
    if let Outer::X(Tagged::A(b)) = &mut o {
        unsafe { (b as *mut bool as *mut u8).write(7) };
    }
    let _o = unsafe { (&o as *const Outer).read() };
}

#[kani::proof]
#[kani::unwind(5)]
fn uninhabited_variant_should_fail() {
    // Write `1`, the discriminant of `B`, to every possible position of the tag.
    let idx: usize = kani::any_where(|idx| *idx < size_of::<WithUninhabited>());
    let _e = with_byte(WithUninhabited::A(0), idx, 1);
}

#[kani::proof]
fn read_after_write_bytes_should_fail() {
    let mut e = Tagged::A(true);
    let p = &mut e as *mut Tagged;
    unsafe {
        std::ptr::write_bytes(p as *mut u8, 0xFF, size_of::<Tagged>());
        let _v = p.read();
    }
}

#[kani::proof]
fn read_after_copy_valid() {
    let src: Tagged = kani::any();
    let mut dst = Tagged::C(0);
    unsafe {
        std::ptr::copy_nonoverlapping(
            &src as *const Tagged as *const u8,
            &mut dst as *mut Tagged as *mut u8,
            size_of::<Tagged>(),
        );
    }
    assert!(matches!(
        (src, dst),
        (Tagged::A(_), Tagged::A(_)) | (Tagged::B(_), Tagged::B(_)) | (Tagged::C(_), Tagged::C(_))
    ));
}
