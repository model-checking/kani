// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Dummy code that gets injected into `core` for basic tests for `verify-std` subcommand.
#[cfg(kani)]
kani_core::kani_lib!(core);

#[cfg(kani)]
#[unstable(feature = "kani", issue = "none")]
pub mod verify {
    use crate::kani;
    use core::num::NonZeroU8;

    #[kani::proof]
    pub fn harness() {
        kani::assert(true, "yay");
    }

    #[kani::proof_for_contract(fake_function)]
    fn dummy_proof() {
        fake_function(true);
    }

    /// Add a `rustc_diagnostic_item` to ensure this works.
    /// See <https://github.com/model-checking/kani/issues/3251> for more details.
    #[kani::requires(x == true)]
    #[rustc_diagnostic_item = "fake_function"]
    fn fake_function(x: bool) -> bool {
        x
    }

    #[kani::proof_for_contract(dummy_read)]
    #[cfg(not(uninit_checks))]
    fn check_dummy_read() {
        let val: char = kani::any();
        assert_eq!(unsafe { dummy_read(&val) }, val);
    }

    /// Ensure we can verify constant functions.
    #[kani::requires(kani::mem::can_dereference(ptr))]
    #[rustc_diagnostic_item = "dummy_read"]
    #[cfg(not(uninit_checks))]
    const unsafe fn dummy_read<T: Copy>(ptr: *const T) -> T {
        *ptr
    }

    #[cfg(not(uninit_checks))]
    #[kani::proof_for_contract(swap_tuple)]
    fn check_swap_tuple() {
        let initial: (char, NonZeroU8) = kani::any();
        let _swapped = swap_tuple(initial);
    }

    #[cfg(not(uninit_checks))]
    #[kani::ensures(| result | result.0 == second && result.1 == first)]
    fn swap_tuple((first, second): (char, NonZeroU8)) -> (NonZeroU8, char) {
        (second, first)
    }

    #[kani::proof_for_contract(add_one)]
    fn check_add_one() {
        let mut initial: [u32; 4] = kani::any();
        unsafe { add_one(&mut initial) };
    }

    /// Function with a more elaborated contract that uses `old` and `modifies`.
    #[kani::modifies(inout)]
    #[kani::requires(kani::mem::can_dereference(inout)
    && unsafe { inout.as_ref_unchecked().iter().all(| i | * i < u32::MAX) })]
    #[kani::requires(kani::mem::can_write(inout))]
    #[kani::ensures(| result | {
    let (orig, i) = old({
    let i = kani::any_where(| i: & usize | * i < unsafe { inout.len() });
    (unsafe { inout.as_ref_unchecked()[i] }, i)});
    unsafe { inout.as_ref_unchecked()[i] > orig }
    })]
    unsafe fn add_one(inout: *mut [u32]) {
        inout.as_mut_unchecked().iter_mut().for_each(|e| *e += 1)
    }

    /// Test that arbitrary pointer works as expected.
    /// Disable it for uninit checks, since these checks do not support `MaybeUninit` which is used
    /// by the pointer generator.
    #[kani::proof]
    #[cfg(not(uninit_checks))]
    fn check_any_ptr() {
        let mut generator = kani::PointerGenerator::<8>::new();
        let ptr = generator.any_in_bounds::<i32>().ptr;
        assert!(kani::mem::can_write_unaligned(ptr));
    }
}
