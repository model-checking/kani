// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! In-bounds uses of `get_unchecked` and `get_unchecked_mut` must still verify.
//! Companion of `tests/expected/get-unchecked-bounds-check`, see
//! <https://github.com/model-checking/kani/issues/5155>.

#[kani::proof]
fn check_get_unchecked_in_bounds() {
    let mut arr = [0u8; 8];
    let s: &mut [u8] = &mut arr[2..6];
    let i: usize = kani::any();
    kani::assume(i < 4);
    unsafe { *s.get_unchecked_mut(i) = 1 };
    assert_eq!(arr[2 + i], 1);
    let r: &[u8] = &arr[2..6];
    assert_eq!(unsafe { *r.get_unchecked(i) }, 1);
}
