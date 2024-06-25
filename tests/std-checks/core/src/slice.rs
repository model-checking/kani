// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Create wrapper functions to standard library functions that contains their contract.
pub mod contracts {
    use kani::{mem::*, requires};

    #[requires(can_dereference(data))]
    #[requires(is_initialized(data, len))]
    pub unsafe fn from_raw_parts<'a, T>(data: *const T, len: usize) -> &'a [T] {
        std::slice::from_raw_parts(data, len)
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    const LEN_MIN: usize = 1;
    const LEN_MAX: usize = 2;

    #[kani::proof_for_contract(contracts::from_raw_parts)]
    #[kani::unwind(25)]
    pub fn check_from_raw_parts_primitive() {
        let len: usize = kani::any();
        kani::assume(len >= LEN_MIN);
        kani::assume(len < LEN_MAX);

        let arr = vec![0u8; len];
        let _slice = unsafe { contracts::from_raw_parts(arr.as_ptr(), len) };
    }
}
