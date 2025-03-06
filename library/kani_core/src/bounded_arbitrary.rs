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

        /// Wraps a type that implements `BoundedArbitrary` along with a bound so that there
        /// is enough information to implement `Arbitrary`.
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # fn foo(x: String) {}
        /// let name: kani::BoundedAny<String, 4> = kani::any();
        /// foo(name.into_inner());
        /// ```
        #[derive(PartialEq, Clone, Debug)]
        pub struct BoundedAny<T, const N: usize>(T);

        impl<T, const N: usize> BoundedAny<T, N> {
            pub fn into_inner(self) -> T {
                self.0
            }
        }

        impl<T, const N: usize> Deref for BoundedAny<T, N> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<T, const N: usize> AsRef<T> for BoundedAny<T, N>
        where
            <BoundedAny<T, N> as Deref>::Target: AsRef<T>,
        {
            fn as_ref(&self) -> &T {
                self.deref()
            }
        }

        impl<T, const N: usize> Arbitrary for BoundedAny<T, N>
        where
            T: BoundedArbitrary,
        {
            fn any() -> Self {
                BoundedAny(T::bounded_any::<N>())
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
                let opt: Result<(), ()> = any();
                opt.map(|_| T::bounded_any::<N>()).map_err(|_| E::bounded_any::<N>())
            }
        }
    };
}
