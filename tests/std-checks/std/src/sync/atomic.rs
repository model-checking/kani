// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate kani;

use std::sync::atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize};

/// Create wrapper functions to standard library functions that contains their contract.
pub mod contracts {
    use super::*;
    use kani::{mem::*, requires};

    #[requires(can_dereference(ptr))]
    pub unsafe fn from_ptr_u8<'a>(ptr: *mut u8) -> &'a AtomicU8 {
        AtomicU8::from_ptr(ptr)
    }

    #[requires(can_dereference(ptr))]
    pub unsafe fn from_ptr_u16<'a>(ptr: *mut u16) -> &'a AtomicU16 {
        AtomicU16::from_ptr(ptr)
    }

    #[requires(can_dereference(ptr))]
    pub unsafe fn from_ptr_u32<'a>(ptr: *mut u32) -> &'a AtomicU32 {
        AtomicU32::from_ptr(ptr)
    }

    #[requires(can_dereference(ptr))]
    pub unsafe fn from_ptr_u64<'a>(ptr: *mut u64) -> &'a AtomicU64 {
        AtomicU64::from_ptr(ptr)
    }

    #[requires(can_dereference(ptr))]
    pub unsafe fn from_ptr_usize<'a>(ptr: *mut usize) -> &'a AtomicUsize {
        AtomicUsize::from_ptr(ptr)
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    #[kani::proof_for_contract(contracts::from_ptr_u8)]
    pub fn check_from_ptr_u8() {
        let ptr = unsafe { std::alloc::alloc(std::alloc::Layout::new::<u8>()) as *mut u8 };
        unsafe { ptr.write(kani::any()) };
        let _ = unsafe { contracts::from_ptr_u8(ptr) };
    }

    #[kani::proof_for_contract(contracts::from_ptr_u16)]
    pub fn check_from_ptr_u16() {
        let ptr = unsafe { std::alloc::alloc(std::alloc::Layout::new::<u16>()) as *mut u16 };
        unsafe { ptr.write(kani::any()) };
        let _ = unsafe { contracts::from_ptr_u16(ptr) };
    }

    #[kani::proof_for_contract(contracts::from_ptr_u32)]
    pub fn check_from_ptr_u32() {
        let ptr = unsafe { std::alloc::alloc(std::alloc::Layout::new::<u32>()) as *mut u32 };
        unsafe { ptr.write(kani::any()) };
        let _ = unsafe { contracts::from_ptr_u32(ptr) };
    }

    #[kani::proof_for_contract(contracts::from_ptr_u64)]
    pub fn check_from_ptr_u64() {
        let ptr = unsafe { std::alloc::alloc(std::alloc::Layout::new::<u64>()) as *mut u64 };
        unsafe { ptr.write(kani::any()) };
        let _ = unsafe { contracts::from_ptr_u64(ptr) };
    }

    #[kani::proof_for_contract(contracts::from_ptr_usize)]
    pub fn check_from_ptr_usize() {
        let ptr = unsafe { std::alloc::alloc(std::alloc::Layout::new::<usize>()) as *mut usize };
        unsafe { ptr.write(kani::any()) };
        let _ = unsafe { contracts::from_ptr_usize(ptr) };
    }
}
