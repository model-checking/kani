// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `offset` does not accept a `count` greater than isize::MAX,
// except for ZSTs, c.f. https://github.com/model-checking/kani/issues/3896

#[kani::proof]
fn test_zst() {
    let mut x = ();
    let ptr: *mut () = &mut x as *mut ();
    let count: usize = (isize::MAX as usize) + 1;
    let _ = unsafe { ptr.add(count) };
}

#[kani::proof]
fn test_non_zst() {
    let mut x = 7;
    let ptr: *mut i32 = &mut x as *mut i32;
    let count: usize = (isize::MAX as usize) + 1;
    let _ = unsafe { ptr.add(count) };
}
