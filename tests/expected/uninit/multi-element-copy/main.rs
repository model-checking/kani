// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! A copy of several elements from initialized memory initializes every element of the
//! destination, not just the first one, also when the source and destination overlap.

use std::mem::MaybeUninit;
use std::ptr;

#[kani::proof]
fn copy_many_from_initialized_ok() {
    let src = [7u8; 16];
    let mut dst = MaybeUninit::<[u8; 16]>::uninit();
    let dst = dst.as_mut_ptr() as *mut u8;
    unsafe { ptr::copy_nonoverlapping(src.as_ptr(), dst, 16) };
    let i: usize = kani::any_where(|i| *i < 16);
    assert_eq!(unsafe { dst.add(i).read() }, 7);
}

#[kani::proof]
fn copy_some_then_read_past_ub() {
    let src = [7u8; 16];
    let mut dst = MaybeUninit::<[u8; 16]>::uninit();
    let dst = dst.as_mut_ptr() as *mut u8;
    unsafe { ptr::copy_nonoverlapping(src.as_ptr(), dst, 8) };
    let _ = unsafe { dst.add(8).read() };
}

/// An overlapping copy within one buffer whose first 8 bytes are initialized.
#[kani::proof]
fn overlapping_copy_ok() {
    let mut buf = [MaybeUninit::<u8>::uninit(); 16];
    for slot in buf.iter_mut().take(8) {
        *slot = MaybeUninit::new(kani::any());
    }
    let p = buf.as_mut_ptr() as *mut u8;
    let shift: usize = kani::any_where(|s| *s <= 8);
    // Move the 8 initialized bytes up by `shift`: they overlap their old place if `shift < 8`.
    unsafe { ptr::copy(p, p.add(shift), 8) };
    let i: usize = kani::any_where(|i| *i < 8);
    let _ = unsafe { p.add(shift + i).read() };
}

/// The same copy, then a read of a byte that was copied from an uninitialized one.
#[kani::proof]
fn overlapping_copy_of_uninit_ub() {
    let mut buf = [MaybeUninit::<u8>::uninit(); 16];
    for slot in buf.iter_mut().take(8) {
        *slot = MaybeUninit::new(kani::any());
    }
    let p = buf.as_mut_ptr() as *mut u8;
    // Bytes 8..12 are uninitialized; copying 12 bytes up by 2 puts byte 8 at 10.
    unsafe { ptr::copy(p, p.add(2), 12) };
    let _ = unsafe { p.add(10).read() };
}
