// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Verify a few std::ptr::NonNull functions.

/// Create wrapper functions to standard library functions that contains their contract.
pub mod contracts {
    /// Swaps the values at two mutable locations, without deinitializing either one.
    ///
    /// TODO: Once history variable has been added, add a post-condition.
    /// Also add a function to do a bitwise value comparison between arguments (ignore paddings).
    ///
    /// ```ignore
    /// #[kani::ensures(kani::equals(kani::old(x), y))]
    /// #[kani::ensures(kani::equals(kani::old(y), x))]
    /// ```
    #[kani::modifies(x)]
    #[kani::modifies(y)]
    pub fn swap<T>(x: &mut T, y: &mut T) {
        std::mem::swap(x, y)
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    #[derive(kani::Arbitrary)]
    struct CannotDrop<T> {
        inner: T,
    }

    impl<T> Drop for CannotDrop<T> {
        fn drop(&mut self) {
            unreachable!("Cannot drop")
        }
    }

    #[kani::proof_for_contract(contracts::swap)]
    pub fn check_swap_primitive() {
        let mut x: u8 = kani::any();
        let mut y: u8 = kani::any();
        contracts::swap(&mut x, &mut y)
    }

    /// FIX-ME: Modifies clause fail with pointer to ZST.
    /// <https://github.com/model-checking/kani/issues/3181>
    /// FIX-ME: `typed_swap` intrisic fails for ZST.
    /// <https://github.com/model-checking/kani/issues/3182>
    #[kani::proof_for_contract(contracts::swap)]
    pub fn check_swap_unit() {
        let mut x: () = kani::any();
        let mut y: () = kani::any();
        contracts::swap(&mut x, &mut y)
    }

    #[kani::proof_for_contract(contracts::swap)]
    pub fn check_swap_adt_no_drop() {
        let mut x: CannotDrop<u8> = kani::any();
        let mut y: CannotDrop<u8> = kani::any();
        contracts::swap(&mut x, &mut y);
        std::mem::forget(x);
        std::mem::forget(y);
    }

    /// Memory swap logic is optimized according to the size and alignment of a type.
    #[kani::proof_for_contract(contracts::swap)]
    pub fn check_swap_large_adt_no_drop() {
        let mut x: CannotDrop<[u128; 5]> = kani::any();
        let mut y: CannotDrop<[u128; 5]> = kani::any();
        contracts::swap(&mut x, &mut y);
        std::mem::forget(x);
        std::mem::forget(y);
    }
}
