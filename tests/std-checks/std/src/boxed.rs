// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate kani;

/// Create wrapper functions to standard library functions that contains their contract.
pub mod contracts {
    use kani::{mem::*, requires};

    /// The actual pre-condition is more complicated: 
    /// 
    /// "For non-zero-sized values, ... a value: *mut T that has been allocated with the Global
    /// allocator with Layout::for_value(&*value) may be converted into a box using
    /// Box::<T>::from_raw(value)."
    ///
    /// "For zero-sized values, the Box pointer still has to be valid for reads and writes and
    /// sufficiently aligned."
    #[requires(can_dereference(raw))]
    pub unsafe fn from_raw<T>(raw: *mut T) -> Box<T> {
        std::boxed::Box::from_raw(raw)
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    #[kani::proof_for_contract(contracts::from_raw)]
    pub fn check_from_raw_u32() {
        let ptr = unsafe { std::alloc::alloc(std::alloc::Layout::new::<u32>()) as *mut u32 };
        unsafe { ptr.write(kani::any()) };
        let _ = unsafe { contracts::from_raw(ptr) };
    }

    #[kani::proof_for_contract(contracts::from_raw)]
    pub fn check_from_raw_unit() {
        let ptr = kani::any::<usize>() as *mut ();
        let _ = unsafe { contracts::from_raw(ptr) };
    }
}
