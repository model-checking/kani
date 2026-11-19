// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check structs whose slice tail starts at an offset that is not a multiple of the struct's
//! alignment. Kani used to emit a padding component after the flexible array that represents the
//! tail, which crashed the compiler: <https://github.com/model-checking/kani/issues/5027>.

/// `a` ends at offset 2 and `c` at 3, so the tail starts at 3 while the struct is 2-aligned.
struct S<T: ?Sized> {
    a: u16,
    c: u8,
    b: T,
}

/// `w` ends at 4 and `v` at 5, so `inner` starts at 6 and its sized part ends at 9, which is not
/// a multiple of the outer struct's 4-byte alignment either.
struct Outer<T: ?Sized> {
    w: u32,
    v: u8,
    inner: S<T>,
}

#[kani::proof]
fn check_slice_tail_after_odd_offset() {
    let s: &S<[u8]> = &S { a: 1, c: 2, b: [3u8; 4] };
    assert_eq!(std::mem::size_of_val(s), 8);
    assert_eq!(std::mem::align_of_val(s), 2);
    assert_eq!(s.a, 1);
    assert_eq!(s.c, 2);
    assert_eq!(s.b.len(), 4);
    assert_eq!(s.b[3], 3);
}

#[kani::proof]
fn check_nested_slice_tail_after_odd_offset() {
    let o: &Outer<[u8]> = &Outer { w: 7, v: 9, inner: S { a: 1, c: 2, b: [3u8; 4] } };
    assert_eq!(std::mem::size_of_val(o), 16);
    assert_eq!(std::mem::align_of_val(o), 4);
    assert_eq!(o.w, 7);
    assert_eq!(o.v, 9);
    assert_eq!(o.inner.a, 1);
    assert_eq!(o.inner.b.len(), 4);
    assert_eq!(o.inner.b[0], 3);
}
