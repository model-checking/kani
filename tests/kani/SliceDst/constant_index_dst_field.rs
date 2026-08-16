// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that slice patterns (which generate `ConstantIndex` projections) work on a slice that
// is the trailing field of an unsized ADT. Such a slice place is a flexible array member
// rather than a pointer dereference, which used to make codegen panic with
// "BinaryOperation Expression does not typecheck Plus" (e.g. on `Arc<[u8]>`'s `ArcInner`,
// as hit by the regex crate).

pub struct Inner<T: ?Sized> {
    pub rc: usize,
    pub data: T,
}

// `[x, y, ..]` generates ConstantIndex { offset, from_end: false } projections.
pub fn first_two(p: &Inner<[u8]>) -> (u8, u8) {
    match p.data {
        [x, y, ..] => (x, y),
        _ => (0, 0),
    }
}

// `[.., y]` generates a ConstantIndex { from_end: true } projection.
pub fn last(p: &Inner<[u8]>) -> u8 {
    match p.data {
        [.., y] => y,
        _ => 0,
    }
}

// Also exercise the same patterns through `Arc<[u8]>`, the standard-library case.
pub fn arc_first(a: &std::sync::Arc<[u8]>) -> u8 {
    match **a {
        [x, ..] => x,
        _ => 0,
    }
}

#[kani::proof]
fn check_dst_slice_patterns() {
    let concrete: Inner<[u8; 3]> = Inner { rc: 1, data: [1, 2, 3] };
    let unsized_ref: &Inner<[u8]> = &concrete;
    let (x, y) = first_two(unsized_ref);
    assert!(x == 1 && y == 2);
    assert!(last(unsized_ref) == 3);

    let a: std::sync::Arc<[u8]> = std::sync::Arc::from([7u8, 8].as_slice());
    assert!(arc_first(&a) == 7);
}
