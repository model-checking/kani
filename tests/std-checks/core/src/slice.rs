// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate kani;

/// Create wrapper functions to standard library functions that contains their contract.
pub mod contracts {
    use kani::{mem::*, requires};

    #[requires(can_dereference(std::ptr::slice_from_raw_parts(data, len)))]
    pub unsafe fn from_raw_parts<'a, T>(data: *const T, len: usize) -> &'a [T] {
        std::slice::from_raw_parts(data, len)
    }

    #[requires(can_dereference(std::ptr::slice_from_raw_parts(data, len)))]
    pub unsafe fn from_raw_parts_mut<'a, T>(data: *mut T, len: usize) -> &'a mut [T] {
        std::slice::from_raw_parts_mut(data, len)
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    const MAX_LEN: usize = isize::MAX as usize;

    #[kani::proof_for_contract(contracts::from_raw_parts)]
    pub fn check_from_raw_parts_primitive() {
        let len: usize = kani::any();
        kani::assume(len < MAX_LEN);

        let arr = vec![0u8; len];
        let _slice = unsafe { contracts::from_raw_parts(arr.as_ptr(), len) };
    }

    #[kani::proof_for_contract(contracts::from_raw_parts_mut)]
    pub fn check_from_raw_parts_mut_primitive() {
        let len: usize = kani::any();
        kani::assume(len < MAX_LEN);

        let mut arr = vec![0u8; len];
        let _slice = unsafe { contracts::from_raw_parts_mut(arr.as_mut_ptr(), len) };
    }
}
