// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This crate is a macro_only crate. It is designed to be used in `no_core` and `no_std`
//! environment.
//!
//! It will contain macros that generate core components of Kani.
//!
//! For regular usage, the kani library will invoke these macros to generate its components as if
//! they were declared in that library.
//!
//! For `no_core` and `no_std` crates, they will have to directly invoke those macros inside a
//! `kani` module in order to generate all the required components.
//! I.e., the components will be part of the crate being compiled.
//!
//! Note that any crate level attribute should be added by kani_driver as RUSTC_FLAGS.
//! E.g.: `register_tool(kanitool)`

#![feature(no_core)]
#![no_core]
#![feature(f16)]
#![feature(f128)]

mod arbitrary;
mod mem;

pub use kani_macros::*;

/// Users should only need to invoke this.
///
/// Options are:
/// - `kani`: Add definitions needed for Kani library.
/// - `core`: Define a `kani` module inside `core` crate.
/// - `std`: TODO: Define a `kani` module inside `std` crate. Users must define kani inside core.
#[macro_export]
macro_rules! kani_lib {
    (core) => {
        #[cfg(kani)]
        #[unstable(feature = "kani", issue = "none")]
        pub mod kani {
            // We need to list them all today because there is conflict with unstable.
            pub use kani_core::*;
            kani_core::kani_intrinsics!(core);
            kani_core::generate_arbitrary!(core);

            pub mod mem {
                kani_core::kani_mem!(core);
            }
        }
    };

    (kani) => {
        pub use kani_core::*;
        kani_core::kani_intrinsics!(std);
        kani_core::generate_arbitrary!(std);
    };
}

/// Kani intrinsics contains the public APIs used by users to verify their harnesses.
/// This macro is a part of kani_core as that allows us to verify even libraries that are no_core
/// such as core in rust's std library itself.
///
/// TODO: Use this inside kani library so that we dont have to maintain two copies of the same intrinsics.
#[macro_export]
macro_rules! kani_intrinsics {
    ($core:tt) => {
        /// Creates an assumption that will be valid after this statement run. Note that the assumption
        /// will only be applied for paths that follow the assumption. If the assumption doesn't hold, the
        /// program will exit successfully.
        ///
        /// # Example:
        ///
        /// The code snippet below should never panic.
        ///
        /// ```rust
        /// let i : i32 = kani::any();
        /// kani::assume(i > 10);
        /// if i < 0 {
        ///   panic!("This will never panic");
        /// }
        /// ```
        ///
        /// The following code may panic though:
        ///
        /// ```rust
        /// let i : i32 = kani::any();
        /// assert!(i < 0, "This may panic and verification should fail.");
        /// kani::assume(i > 10);
        /// ```
        #[inline(never)]
        #[rustc_diagnostic_item = "KaniAssume"]
        #[cfg(not(feature = "concrete_playback"))]
        pub fn assume(cond: bool) {
            let _ = cond;
        }

        #[inline(never)]
        #[rustc_diagnostic_item = "KaniAssume"]
        #[cfg(feature = "concrete_playback")]
        pub fn assume(cond: bool) {
            assert!(cond, "`kani::assume` should always hold");
        }

        /// Creates an assertion of the specified condition and message.
        ///
        /// # Example:
        ///
        /// ```rust
        /// let x: bool = kani::any();
        /// let y = !x;
        /// kani::assert(x || y, "ORing a boolean variable with its negation must be true")
        /// ```
        #[cfg(not(feature = "concrete_playback"))]
        #[inline(never)]
        #[rustc_diagnostic_item = "KaniAssert"]
        pub const fn assert(cond: bool, msg: &'static str) {
            let _ = cond;
            let _ = msg;
        }

        #[cfg(feature = "concrete_playback")]
        #[inline(never)]
        #[rustc_diagnostic_item = "KaniAssert"]
        pub const fn assert(cond: bool, msg: &'static str) {
            assert!(cond, "{}", msg);
        }

        /// Creates an assertion of the specified condition and message, but does not assume it afterwards.
        ///
        /// # Example:
        ///
        /// ```rust
        /// let x: bool = kani::any();
        /// let y = !x;
        /// kani::check(x || y, "ORing a boolean variable with its negation must be true")
        /// ```
        #[cfg(not(feature = "concrete_playback"))]
        #[inline(never)]
        #[rustc_diagnostic_item = "KaniCheck"]
        pub const fn check(cond: bool, msg: &'static str) {
            let _ = cond;
            let _ = msg;
        }

        #[cfg(feature = "concrete_playback")]
        #[inline(never)]
        #[rustc_diagnostic_item = "KaniCheck"]
        pub const fn check(cond: bool, msg: &'static str) {
            assert!(cond, "{}", msg);
        }

        /// Creates a cover property with the specified condition and message.
        ///
        /// # Example:
        ///
        /// ```rust
        /// kani::cover(slice.len() == 0, "The slice may have a length of 0");
        /// ```
        ///
        /// A cover property checks if there is at least one execution that satisfies
        /// the specified condition at the location in which the function is called.
        ///
        /// Cover properties are reported as:
        ///  - SATISFIED: if Kani found an execution that satisfies the condition
        ///  - UNSATISFIABLE: if Kani proved that the condition cannot be satisfied
        ///  - UNREACHABLE: if Kani proved that the cover property itself is unreachable (i.e. it is vacuously UNSATISFIABLE)
        ///
        /// This function is called by the [`cover!`] macro. The macro is more
        /// convenient to use.
        ///
        #[inline(never)]
        #[rustc_diagnostic_item = "KaniCover"]
        pub const fn cover(_cond: bool, _msg: &'static str) {}

        /// This creates an symbolic *valid* value of type `T`. You can assign the return value of this
        /// function to a variable that you want to make symbolic.
        ///
        /// # Example:
        ///
        /// In the snippet below, we are verifying the behavior of the function `fn_under_verification`
        /// under all possible `NonZeroU8` input values, i.e., all possible `u8` values except zero.
        ///
        /// ```rust
        /// let inputA = kani::any::<std::num::NonZeroU8>();
        /// fn_under_verification(inputA);
        /// ```
        ///
        /// Note: This is a safe construct and can only be used with types that implement the `Arbitrary`
        /// trait. The Arbitrary trait is used to build a symbolic value that represents all possible
        /// valid values for type `T`.
        #[rustc_diagnostic_item = "KaniAny"]
        #[inline(always)]
        pub fn any<T: Arbitrary>() -> T {
            T::any()
        }

        /// This function is only used for function contract instrumentation.
        /// It behaves exaclty like `kani::any<T>()`, except it will check for the trait bounds
        /// at compilation time. It allows us to avoid type checking errors while using function
        /// contracts only for verification.
        #[rustc_diagnostic_item = "KaniAnyModifies"]
        #[inline(never)]
        #[doc(hidden)]
        pub fn any_modifies<T>() -> T {
            // This function should not be reacheable.
            // Users must include `#[kani::recursion]` in any function contracts for recursive functions;
            // otherwise, this might not be properly instantiate. We mark this as unreachable to make
            // sure Kani doesn't report any false positives.
            unreachable!()
        }

        /// This creates a symbolic *valid* value of type `T`.
        /// The value is constrained to be a value accepted by the predicate passed to the filter.
        /// You can assign the return value of this function to a variable that you want to make symbolic.
        ///
        /// # Example:
        ///
        /// In the snippet below, we are verifying the behavior of the function `fn_under_verification`
        /// under all possible `u8` input values between 0 and 12.
        ///
        /// ```rust
        /// let inputA: u8 = kani::any_where(|x| *x < 12);
        /// fn_under_verification(inputA);
        /// ```
        ///
        /// Note: This is a safe construct and can only be used with types that implement the `Arbitrary`
        /// trait. The Arbitrary trait is used to build a symbolic value that represents all possible
        /// valid values for type `T`.
        #[inline(always)]
        pub fn any_where<T: Arbitrary, F: FnOnce(&T) -> bool>(f: F) -> T {
            let result = T::any();
            assume(f(&result));
            result
        }

        /// This function creates a symbolic value of type `T`. This may result in an invalid value.
        ///
        /// # Safety
        ///
        /// This function is unsafe and it may represent invalid `T` values which can lead to many
        /// undesirable undefined behaviors. Because of that, this function can only be used
        /// internally when we can guarantee that the type T has no restriction regarding its bit level
        /// representation.
        ///
        /// This function is also used to find concrete values in the CBMC output trace
        /// and return those concrete values in concrete playback mode.
        ///
        /// Note that SIZE_T must be equal the size of type T in bytes.
        #[inline(never)]
        #[cfg(not(feature = "concrete_playback"))]
        pub(crate) unsafe fn any_raw_internal<T, const SIZE_T: usize>() -> T {
            any_raw_inner::<T>()
        }

        #[inline(never)]
        #[cfg(feature = "concrete_playback")]
        pub(crate) unsafe fn any_raw_internal<T, const SIZE_T: usize>() -> T {
            concrete_playback::any_raw_internal::<T, SIZE_T>()
        }

        /// This low-level function returns nondet bytes of size T.
        #[rustc_diagnostic_item = "KaniAnyRaw"]
        #[inline(never)]
        #[allow(dead_code)]
        pub fn any_raw_inner<T>() -> T {
            kani_intrinsic()
        }

        /// Function used to generate panic with a static message as this is the only one currently
        /// supported by Kani display.
        ///
        /// During verification this will get replaced by `assert(false)`. For concrete executions, we just
        /// invoke the regular `std::panic!()` function. This function is used by our standard library
        /// overrides, but not the other way around.
        #[inline(never)]
        #[rustc_diagnostic_item = "KaniPanic"]
        #[doc(hidden)]
        pub const fn panic(message: &'static str) -> ! {
            panic!("{}", message)
        }

        /// An empty body that can be used to define Kani intrinsic functions.
        ///
        /// A Kani intrinsic is a function that is interpreted by Kani compiler.
        /// While we could use `unreachable!()` or `panic!()` as the body of a kani intrinsic
        /// function, both cause Kani to produce a warning since we don't support caller location.
        /// (see https://github.com/model-checking/kani/issues/2010).
        ///
        /// This function is dead, since its caller is always  handled via a hook anyway,
        /// so we just need to put a body that rustc does not complain about.
        /// An infinite loop works out nicely.
        fn kani_intrinsic<T>() -> T {
            #[allow(clippy::empty_loop)]
            loop {}
        }

        pub mod internal {

            /// Helper trait for code generation for `modifies` contracts.
            ///
            /// We allow the user to provide us with a pointer-like object that we convert as needed.
            #[doc(hidden)]
            pub trait Pointer<'a> {
                /// Type of the pointed-to data
                type Inner;

                /// Used for checking assigns contracts where we pass immutable references to the function.
                ///
                /// We're using a reference to self here, because the user can use just a plain function
                /// argument, for instance one of type `&mut _`, in the `modifies` clause which would move it.
                unsafe fn decouple_lifetime(&self) -> &'a Self::Inner;

                /// used for havocking on replecement of a `modifies` clause.
                unsafe fn assignable(self) -> &'a mut Self::Inner;
            }

            impl<'a, 'b, T> Pointer<'a> for &'b T {
                type Inner = T;
                unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
                    $core::mem::transmute(*self)
                }

                #[allow(clippy::transmute_ptr_to_ref)]
                unsafe fn assignable(self) -> &'a mut Self::Inner {
                    $core::mem::transmute(self as *const T)
                }
            }

            impl<'a, 'b, T> Pointer<'a> for &'b mut T {
                type Inner = T;

                #[allow(clippy::transmute_ptr_to_ref)]
                unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
                    $core::mem::transmute::<_, &&'a T>(self)
                }

                unsafe fn assignable(self) -> &'a mut Self::Inner {
                    $core::mem::transmute(self)
                }
            }

            impl<'a, T> Pointer<'a> for *const T {
                type Inner = T;
                unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
                    &**self as &'a T
                }

                #[allow(clippy::transmute_ptr_to_ref)]
                unsafe fn assignable(self) -> &'a mut Self::Inner {
                    $core::mem::transmute(self)
                }
            }

            impl<'a, T> Pointer<'a> for *mut T {
                type Inner = T;
                unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
                    &**self as &'a T
                }

                #[allow(clippy::transmute_ptr_to_ref)]
                unsafe fn assignable(self) -> &'a mut Self::Inner {
                    $core::mem::transmute(self)
                }
            }

            /// A way to break the ownerhip rules. Only used by contracts where we can
            /// guarantee it is done safely.
            #[inline(never)]
            #[doc(hidden)]
            #[rustc_diagnostic_item = "KaniUntrackedDeref"]
            pub fn untracked_deref<T>(_: &T) -> T {
                todo!()
            }

            /// CBMC contracts currently has a limitation where `free` has to be in scope.
            /// However, if there is no dynamic allocation in the harness, slicing removes `free` from the
            /// scope.
            ///
            /// Thus, this function will basically translate into:
            /// ```c
            /// // This is a no-op.
            /// free(NULL);
            /// ```
            #[inline(never)]
            #[doc(hidden)]
            #[rustc_diagnostic_item = "KaniInitContracts"]
            pub fn init_contracts() {}
        }
    };
}
