// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Constant fat pointers to custom slice-tailed DSTs. The provenance-carrying forms below are
//! supported and must keep verifying. (The provenance-free form -- a dangling aligned data
//! pointer, as produced by zerovec's `ZeroSlice::new_empty()` and reached via the idna crate
//! -- previously crashed the compiler in `codegen_const_ptr` and now degrades to an
//! unsupported-construct check; it is not readily reducible to a standalone test, so it is
//! covered by the idna reproduction in the associated PR.)

pub struct SliceTailed<T: ?Sized> {
    pub head: u8,
    pub tail: T,
}

#[repr(transparent)]
pub struct Transparent<T: ?Sized> {
    pub tail: T,
}

pub static VALUES: &SliceTailed<[u16]> = &SliceTailed { head: 1, tail: [2, 3, 4] };

// Mimics zerovec's ZeroSlice::from_ule_slice: a transmuted transparent slice wrapper.
pub static EMPTY: &Transparent<[u16]> =
    unsafe { std::mem::transmute::<&[u16], &Transparent<[u16]>>(&[]) };

#[kani::proof]
fn check_dst_consts() {
    assert_eq!(VALUES.head, 1);
    assert_eq!(VALUES.tail.len(), 3);
    assert_eq!(EMPTY.tail.len(), 0);
}
