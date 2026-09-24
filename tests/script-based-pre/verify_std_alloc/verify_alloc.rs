// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Code injected into `alloc` for the `verify_std_alloc` test: the Kani definitions that need an
/// allocator, a proof that uses their implementations, and two functions whose automatic
/// harnesses need their models.
#[cfg(kani)]
kani_core::kani_lib!(alloc);

#[cfg(kani)]
#[unstable(feature = "kani", issue = "none")]
pub mod verify_alloc {
    use crate::boxed::Box;
    use crate::vec::Vec;
    use core::kani;

    /// Needs the `Arbitrary` and `BoundedArbitrary` implementations from `kani_lib!(alloc)`;
    /// without them it does not compile.
    #[kani::proof]
    fn check_alloc_arbitrary() {
        let _b: Box<u8> = kani::any();
        let v: Vec<u8> = kani::bounded_any::<Vec<u8>, 4>();
        assert!(v.len() <= 4);
    }

    /// Has no `Arbitrary` implementation, so autoharness derives one.
    pub struct Pair(pub u8, pub u8);

    /// Its harness needs `any_box` from `alloc`, since `Pair` is derived rather than implemented.
    pub fn first(p: Box<Pair>) -> u8 {
        p.0
    }

    /// Its harness needs `any_vec_unbounded` from `alloc`.
    pub fn len(v: Vec<u8>) -> usize {
        v.len()
    }
}
