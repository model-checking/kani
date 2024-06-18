// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Add contracts for functions from std::ptr.

use std::ptr::NonNull;

/// Create wrapper functions to standard library functions that contains their contract.
pub mod contracts {
    use super::*;
    use kani::{ensures, implies, mem::*, modifies, requires};

    #[ensures(|result : &Option<NonNull<T>>| implies!(ptr.is_null() => result.is_none()))]
    #[ensures(|result : &Option<NonNull<T>>| implies!(!ptr.is_null() => result.is_some()))]
    pub fn new<T>(ptr: *mut T) -> Option<NonNull<T>> {
        NonNull::new(ptr)
    }

    /// # Safety:
    /// When calling this method, you have to ensure that all the following is true:
    ///
    ///  - The pointer must be properly aligned.
    ///  - It must be “dereferenceable” in the sense defined in the module documentation.
    ///  - TODO: The pointer must point to an initialized instance of T.
    ///     - We check for value validity, but not initialization yet.
    ///
    /// TODO: How to ensure aliasing rules??
    /// You must enforce Rust’s aliasing rules, since the returned lifetime 'a is arbitrarily chosen and does not
    /// necessarily reflect the actual lifetime of the data. In particular, while this reference exists, the memory
    /// the pointer points to must not get mutated (except inside UnsafeCell).
    /// Taken from: <https://doc.rust-lang.org/std/ptr/struct.NonNull.html#method.as_ref>
    #[requires(can_dereference(obj.as_ptr()))]
    #[requires(is_initialized(obj.as_ptr()))]
    pub unsafe fn as_ref<'a, T>(obj: &NonNull<T>) -> &'a T {
        obj.as_ref()
    }

    #[requires(!ptr.is_null())]
    pub unsafe fn new_unchecked<T>(ptr: *mut T) -> NonNull<T> {
        NonNull::<T>::new_unchecked(ptr)
    }

    /// Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///   - `dst` must be valid for both reads and writes.
    ///   - `dst` must be properly aligned.
    ///   - TODO: `dst` must point to a properly initialized value of type `T`.
    ///     - We check validity but not initialization.
    ///
    /// Note that even if `T` has size 0, the pointer must be non-null and properly aligned.
    #[requires(can_dereference(dst))]
    #[requires(is_initialized(dst))]
    #[modifies(dst)]
    pub unsafe fn replace<T>(dst: *mut T, src: T) -> T {
        std::ptr::replace(dst, src)
    }
}

#[cfg(kani)]
mod verify {
    use super::*;
    use kani::cover;

    #[kani::proof_for_contract(contracts::new)]
    pub fn check_new() {
        let ptr = kani::any::<usize>() as *mut ();
        let res = contracts::new(ptr);
        cover!(res.is_none());
        cover!(res.is_some());
    }

    #[kani::proof_for_contract(contracts::new_unchecked)]
    pub fn check_new_unchecked() {
        let ptr = kani::any::<usize>() as *mut ();
        let _ = unsafe { contracts::new_unchecked(ptr) };
    }

    #[kani::proof_for_contract(contracts::as_ref)]
    pub fn check_as_ref() {
        let ptr = kani::any::<Box<usize>>();
        let non_null = NonNull::new(Box::into_raw(ptr)).unwrap();
        let _rf = unsafe { contracts::as_ref(&non_null) };
    }

    #[kani::proof_for_contract(contracts::as_ref)]
    #[kani::should_panic]
    pub fn check_as_ref_dangling() {
        let ptr = kani::any::<usize>() as *mut u8;
        kani::assume(!ptr.is_null());
        let non_null = NonNull::new(ptr).unwrap();
        let _rf = unsafe { contracts::as_ref(&non_null) };
    }

    /// FIX-ME: Modifies clause fail with pointer to ZST.
    /// <https://github.com/model-checking/kani/issues/3181>
    #[kani::proof_for_contract(contracts::replace)]
    pub fn check_replace_unit() {
        check_replace_impl::<()>();
    }

    #[kani::proof_for_contract(contracts::replace)]
    pub fn check_replace_char() {
        check_replace_impl::<char>();
    }

    fn check_replace_impl<T: kani::Arbitrary + Eq + Clone>() {
        let mut dst = T::any();
        let orig = dst.clone();
        let src = T::any();
        let ret = unsafe { contracts::replace(&mut dst, src.clone()) };
        assert_eq!(ret, orig);
        assert_eq!(dst, src);
    }
}
