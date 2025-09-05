// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the `BoundedArbitrary` trait.

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_bounded_arbitrary {
    () => {
        use core_path::ops::Deref;

        pub trait BoundedArbitrary {
            fn bounded_any<const N: usize>() -> Self;
        }

        impl<T: Arbitrary> BoundedArbitrary for alloc::boxed::Box<[T]> {
            fn bounded_any<const N: usize>() -> Self {
                let len: usize = kani::any_where(|l| *l <= N);
                // The following is equivalent to:
                // ```
                // (0..len).map(|_| T::any()).collect()
                // ```
                // but leads to more efficient verification
                let mut b = alloc::boxed::Box::<[T]>::new_uninit_slice(len);
                for i in 0..len {
                    b[i] = MaybeUninit::new(T::any());
                }
                unsafe { b.assume_init() }
            }
        }

        impl<T> BoundedArbitrary for Option<T>
        where
            T: BoundedArbitrary,
        {
            fn bounded_any<const N: usize>() -> Self {
                let opt: Option<()> = any();
                opt.map(|_| T::bounded_any::<N>())
            }
        }

        impl<T, E> BoundedArbitrary for Result<T, E>
        where
            T: BoundedArbitrary,
            E: BoundedArbitrary,
        {
            fn bounded_any<const N: usize>() -> Self {
                let res: Result<(), ()> = any();
                res.map(|_| T::bounded_any::<N>()).map_err(|_| E::bounded_any::<N>())
            }
        }
    };
}
