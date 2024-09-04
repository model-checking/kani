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

    #[kani::modifies(dest)]
    pub fn replace<T>(dest: &mut T, src: T) -> T {
        std::mem::replace(dest, src)
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    /// Use this type to ensure that mem swap does not drop the value.
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

    #[kani::proof_for_contract(contracts::replace)]
    pub fn check_replace_primitive() {
        let mut x: u8 = kani::any();
        let x_before = x;

        let y: u8 = kani::any();
        let x_returned = contracts::replace(&mut x, y);

        kani::assert(x_before == x_returned, "x_before == x_returned");
    }

    #[kani::proof_for_contract(contracts::replace)]
    pub fn check_replace_adt_no_drop() {
        let mut x: CannotDrop<u8> = kani::any();
        let y: CannotDrop<u8> = kani::any();
        let new_x = contracts::replace(&mut x, y);
        std::mem::forget(x);
        std::mem::forget(new_x);
    }

    /// Memory replace logic is optimized according to the size and alignment of a type.
    #[kani::proof_for_contract(contracts::replace)]
    pub fn check_replace_large_adt_no_drop() {
        let mut x: CannotDrop<[u128; 4]> = kani::any();
        let y: CannotDrop<[u128; 4]> = kani::any();
        let new_x = contracts::replace(&mut x, y);
        std::mem::forget(x);
        std::mem::forget(new_x);
    }
}
