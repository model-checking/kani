// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that offset_from_ptr works for vector and types with size that are not power of two.
#![feature(ptr_sub_ptr)]

#[kani::proof]
#[kani::unwind(5)]
fn offset_from_vec() {
    let v1 = vec![vec![1], vec![2]];
    let it = v1.into_iter();
    assert_eq!(it.size_hint().0, 2);
}

#[kani::proof]
fn offset_non_power_two() {
    let mut v = vec![[0u64; 3], [2u64; 3]];
    unsafe {
        let offset = kani::any_where(|o: &usize| *o <= v.len());
        let begin = v.as_mut_ptr();
        let end = begin.add(offset);
        assert_eq!(end.sub_ptr(begin), offset);
    }
}
