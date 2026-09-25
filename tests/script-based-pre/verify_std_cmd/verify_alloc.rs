// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Gets injected into `alloc` to check that the models that need `alloc`'s own types are
/// available in the `verify-std` flow, c.f. <https://github.com/model-checking/kani/issues/4807>.
/// `core::kani` cannot name `Vec`, `Box` and friends, so the implementations have to live here.
#[cfg(kani)]
kani_core::kani_lib!(alloc);

#[cfg(kani)]
#[unstable(feature = "kani", issue = "none")]
pub mod verify_alloc {
    use crate::kani;
    use crate::vec::Vec;

    /// `Vec<T>: BoundedArbitrary`, which is what automatic harnesses use for a `Vec` argument.
    #[kani::proof]
    fn check_bounded_vec() {
        let v: Vec<u8> = kani::bounded_any::<_, 2>();
        assert!(v.len() <= 2);
    }

    /// `Box<T>: Arbitrary`.
    #[kani::proof]
    fn check_any_box() {
        let b: crate::boxed::Box<u32> = kani::any();
        assert!(*b <= u32::MAX);
    }
}
