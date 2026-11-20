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
