// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// expect-fail
//! Verify a few std::ptr::NonNull functions.

use std::ptr::NonNull;

mod contracts {
    use super::*;

    #[kani::ensures(kani::implies!(ptr.is_null() => result.is_none()))]
    #[kani::ensures(kani::implies!(!ptr.is_null() => result.map_or(false, |non_null| !non_null.as_ptr().is_null())))]
    pub fn new<T>(ptr: *mut T) -> Option<NonNull<T>> {
        NonNull::new(ptr)
    }

    /// # Safety:
    /// When calling this method, you have to ensure that all the following is true:
    ///
    ///  - TODO: The pointer must be properly aligned.
    ///  - It must be “dereferenceable” in the sense defined in the module documentation.
    ///  - TODO: The pointer must point to an initialized instance of T.
    ///
    /// You must enforce Rust’s aliasing rules, since the returned lifetime 'a is arbitrarily chosen and does not
    /// necessarily reflect the actual lifetime of the data. In particular, while this reference exists, the memory
    /// the pointer points to must not get mutated (except inside UnsafeCell).
    /// Taken from: <https://doc.rust-lang.org/std/ptr/struct.NonNull.html#method.as_ref>
    #[kani::requires(kani::mem::expect_valid_ptr(obj.as_ptr()))]
    #[kani::requires(kani::mem::is_ptr_aligned(obj.as_ptr()))]
    pub unsafe fn as_ref<'a, T>(obj: &NonNull<T>) -> &'a T {
        obj.as_ref()
    }

    #[kani::requires(!ptr.is_null())]
    #[kani::ensures(!result.as_ptr().is_null())]
    pub unsafe fn new_unchecked<T>(ptr: *mut T) -> NonNull<T> {
        NonNull::<T>::new_unchecked(ptr)
    }
}

#[kani::proof_for_contract(contracts::new)]
pub fn check_new() {
    let ptr = kani::any::<usize>() as *mut ();
    let res = contracts::new(ptr);
    kani::cover!(res.is_none());
    kani::cover!(res.is_some());
}

#[kani::proof_for_contract(contracts::new_unchecked)]
pub fn check_new_unchecked() {
    let ptr = kani::any::<usize>() as *mut ();
    let _ = unsafe { contracts::new_unchecked(ptr) };
}

#[kani::proof_for_contract(contracts::as_ref)]
pub fn check_as_ref() {
    let ptr = kani::any::<usize>() as *mut u8;
    kani::assume(!ptr.is_null());
    let Some(non_null) = NonNull::new(ptr) else {
        unreachable!();
    };
    let _rf = unsafe { contracts::as_ref(&non_null) };
}
