// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check how Kani handles arrays that are zero sized.

pub fn first<T>(slice: &[T]) -> Option<&T> {
    slice.first()
}

#[kani::proof]
pub fn check_zero_elems() {
    let empty_array: [u8; 0] = kani::any();
    assert_eq!(empty_array.len(), 0);

    assert_eq!(first(&empty_array), None);

    let cloned = empty_array.clone();
    assert_eq!(cloned, empty_array);

    let moved = empty_array;
    assert_eq!(moved, cloned);

    for _ in empty_array {
        unreachable!("No iteration should be possible");
    }
}

#[kani::proof]
#[kani::unwind(11)]
pub fn check_zst_elem() {
    let zst_array: [(); 10] = kani::any();
    assert_eq!(zst_array.len(), 10);

    assert_eq!(first(&zst_array), Some(&()));

    let cloned = zst_array.clone();
    assert_eq!(cloned, zst_array);

    let moved = zst_array;
    assert_eq!(moved, cloned);

    for e in zst_array {
        assert_eq!(e, ());
    }
}

#[kani::proof]
pub fn check_zst_enum() {
    #[derive(kani::Arbitrary)]
    enum ZeroSz {
        Empty([u8; 0]),
        ZeroSzElem([(); 10]),
    }

    let zst: ZeroSz = kani::any();
    match zst {
        ZeroSz::Empty(arr) => assert_eq!(arr.len(), 0),
        ZeroSz::ZeroSzElem(arr) => assert_eq!(arr.len(), 10),
    }
}
