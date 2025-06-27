// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
//! Add a check to ensure that we correctly detect reading the padding values of a static and
//! also of a constant.

#![feature(generic_const_exprs)]

/// Check if all the values in the buffer is equals to zero.
unsafe fn is_zeroed<T>(orig: *const T) -> bool
where
    [(); size_of::<T>()]:,
{
    let buf = orig as *const [u8; size_of::<T>()];
    unsafe { &*buf }.iter().all(|val| *val == 0)
}

const CONST_PADDING: (u8, u16) = (0, 0);
static STATIC_PADDING: (u8, char) = (0, '\0');

#[kani::proof]
fn check_read_const_padding_fails() {
    assert!(unsafe { is_zeroed(&CONST_PADDING) });
}

#[kani::proof]
fn check_read_static_padding_fails() {
    assert!(unsafe { is_zeroed(&STATIC_PADDING) });
}

#[kani::proof]
fn check_read_assoc_const_padding_fails() {
    assert!(unsafe { is_zeroed(&(0u128, 0u16)) });
}
