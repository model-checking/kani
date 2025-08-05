// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
//! Test function contracts on generic trait implementations based on SliceIndex,
//! c.f. https://github.com/model-checking/kani/issues/4084
//! This `proof_for_contract` should work,
//! but we do not yet support stubbing/contracts on trait fns with generic arguments
//! c.f. https://github.com/model-checking/kani/issues/1997#issuecomment-3134614734.
//! So for now, test that we emit a nice error message.

const INVALID_INDEX: usize = 10;

trait SliceIndex<T: ?Sized> {
    type Output: ?Sized;
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output;
}

impl<T> SliceIndex<[T]> for usize {
    type Output = T;

    #[kani::requires(!slice.is_null())]
    #[kani::requires(self < slice.len())]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const Self::Output {
        unsafe { (*slice).as_ptr().add(self) }
    }
}

#[kani::proof_for_contract(<usize as SliceIndex<[i32]>>::get_unchecked)]
fn test_generic_slice_contract() {
    let data = [1i32, 2, 3, 4, 5];
    let slice_ptr = &data as *const [i32];

    unsafe {
        // This violates the contract precondition (index >= slice length)
        let _ptr = INVALID_INDEX.get_unchecked(slice_ptr);
    }
}
