// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

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
