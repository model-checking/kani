// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::arbitrary::Arbitrary;

/// Helper trait for code generation for `modifies` contracts.
///
/// We allow the user to provide us with a pointer-like object that we convert as needed.
#[doc(hidden)]
pub trait Pointer<'a> {
    /// Type of the pointed-to data
    type Inner: ?Sized;

    /// Used for checking assigns contracts where we pass immutable references to the function.
    ///
    /// We're using a reference to self here, because the user can use just a plain function
    /// argument, for instance one of type `&mut _`, in the `modifies` clause which would move it.
    unsafe fn decouple_lifetime(&self) -> &'a Self::Inner;

    unsafe fn assignable(self) -> &'a mut Self::Inner;
}

impl<'a, 'b, T: ?Sized> Pointer<'a> for &'b T {
    type Inner = T;
    unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
        std::mem::transmute(*self)
    }

    #[allow(clippy::transmute_ptr_to_ref)]
    unsafe fn assignable(self) -> &'a mut Self::Inner {
        std::mem::transmute(self as *const T)
    }
}

impl<'a, 'b, T: ?Sized> Pointer<'a> for &'b mut T {
    type Inner = T;

    #[allow(clippy::transmute_ptr_to_ref)]
    unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
        std::mem::transmute::<_, &&'a T>(self)
    }

    unsafe fn assignable(self) -> &'a mut Self::Inner {
        std::mem::transmute(self)
    }
}

impl<'a, T: ?Sized> Pointer<'a> for *const T {
    type Inner = T;
    unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
        &**self as &'a T
    }

    #[allow(clippy::transmute_ptr_to_ref)]
    unsafe fn assignable(self) -> &'a mut Self::Inner {
        std::mem::transmute(self)
    }
}

impl<'a, T: ?Sized> Pointer<'a> for *mut T {
    type Inner = T;
    unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
        &**self as &'a T
    }

    #[allow(clippy::transmute_ptr_to_ref)]
    unsafe fn assignable(self) -> &'a mut Self::Inner {
        std::mem::transmute(self)
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

/// This should only be used within contracts. The intent is to
/// perform type inference on a closure's argument
#[doc(hidden)]
pub fn apply_closure<T, U: Fn(&T) -> bool>(f: U, x: &T) -> bool {
    f(x)
}

/// This function is only used for function contract instrumentation.
/// It behaves exaclty like `kani::any<T>()`, except it will check for the trait bounds
/// at compilation time. It allows us to avoid type checking errors while using function
/// contracts only for verification.
#[crate::unstable(feature="function-contracts", issue="none", reason="function-contracts")]
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

/// Recieves a reference to a pointer-like object and assigns kani::any_modifies to that object.
/// Only for use within function contracts and will not be replaced if the recursive or function stub
/// replace contracts are not used.
#[crate::unstable(feature="function-contracts", issue="none", reason="function-contracts")]
#[rustc_diagnostic_item = "KaniWriteAny"]
#[inline(never)]
#[doc(hidden)]
pub fn write_any<T: ?Sized>(_pointer: &T) {
    // This function should not be reacheable.
    // Users must include `#[kani::recursion]` in any function contracts for recursive functions;
    // otherwise, this might not be properly instantiate. We mark this as unreachable to make
    // sure Kani doesn't report any false positives.
    unreachable!()
}

#[crate::unstable(feature="function-contracts", issue="none", reason="function-contracts")]
#[rustc_diagnostic_item = "KaniWriteAnySlice"]
#[inline(always)]
pub fn write_any_slice<T: Arbitrary>(slice: &mut [T]) {
    slice.fill_with(T::any)
}

#[crate::unstable(feature="function-contracts", issue="none", reason="function-contracts")]
#[rustc_diagnostic_item = "KaniWriteAnySlim"]
#[inline(always)]
pub fn write_any_slim<T: Arbitrary>(pointer: &mut T) {
    *pointer = T::any()
}

#[crate::unstable(feature="function-contracts", issue="none", reason="function-contracts")]
#[rustc_diagnostic_item = "KaniWriteAnyStr"]
#[inline(always)]
pub fn write_any_str(s: &mut str) {
    unsafe { s.as_bytes_mut() }.fill_with(u8::any)
    //TODO: String validation
}
