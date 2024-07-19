// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

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
        std::mem::transmute(*self)
    }

    #[allow(clippy::transmute_ptr_to_ref)]
    unsafe fn assignable(self) -> &'a mut Self::Inner {
        std::mem::transmute(self as *const T)
    }
}

impl<'a, 'b, T> Pointer<'a> for &'b mut T {
    type Inner = T;

    #[allow(clippy::transmute_ptr_to_ref)]
    unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
        std::mem::transmute::<_, &&'a T>(self)
    }

    unsafe fn assignable(self) -> &'a mut Self::Inner {
        std::mem::transmute(self)
    }
}

impl<'a, T> Pointer<'a> for *const T {
    type Inner = T;
    unsafe fn decouple_lifetime(&self) -> &'a Self::Inner {
        &**self as &'a T
    }

    #[allow(clippy::transmute_ptr_to_ref)]
    unsafe fn assignable(self) -> &'a mut Self::Inner {
        std::mem::transmute(self)
    }
}

impl<'a, T> Pointer<'a> for *mut T {
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
/// TODO: Remove this! This is not safe. Users should be able to use `ptr::read` and `old` if
/// they really need to.
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
/// TODO: This should be generated inside the function that has contract. This is used for
/// remembers.
#[doc(hidden)]
pub fn apply_closure<T, U: Fn(&T) -> bool>(f: U, x: &T) -> bool {
    f(x)
}

/// Function that calls a closure used to implement contracts.
///
/// In contracts, we cannot invoke the generated closures directly, instead, we call register
/// contract. This function is a no-op. However, in the reality, we do want to call the closure,
/// so we swap the register body by this function body.
#[doc(hidden)]
#[allow(dead_code)]
#[rustc_diagnostic_item = "KaniRunContract"]
fn run_contract_fn<T, F: FnOnce() -> T>(func: F) -> T {
    func()
}

/// This is used for documentation's sake of which implementation to keep during contract verification.
#[doc(hidden)]
type Mode = u8;

/// Keep the original body.
pub const ORIGINAL: Mode = 0;

/// Run the check with recursion support.
pub const RECURSION_CHECK: Mode = 1;

/// Run the simple check with no recursion support.
pub const SIMPLE_CHECK: Mode = 2;

/// Stub the body with its contract.
pub const REPLACE: Mode = 3;

/// This function is only used to help with contract instrumentation.
///
/// It should be removed from the end user code during contract transformation.
/// By default, return the original code (used in concrete playback).
#[doc(hidden)]
#[inline(never)]
#[crate::unstable(
    feature = "function-contracts",
    issue = 2652,
    reason = "experimental support for function contracts"
)]
#[rustc_diagnostic_item = "KaniContractMode"]
pub const fn mode() -> Mode {
    ORIGINAL
}
