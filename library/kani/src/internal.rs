// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::arbitrary::Arbitrary;
use std::ptr;

/// Helper trait for code generation for `modifies` contracts.
///
/// We allow the user to provide us with a pointer-like object that we convert as needed.
#[doc(hidden)]
pub trait Pointer<'a> {
    /// Type of the pointed-to data
    type Inner: ?Sized;

    unsafe fn assignable(self) -> *mut Self::Inner;
}

impl<'a, 'b, T: ?Sized> Pointer<'a> for &'b T {
    type Inner = T;
    unsafe fn assignable(self) -> *mut Self::Inner {
        std::mem::transmute(self as *const T)
    }
}

impl<'a, 'b, T: ?Sized> Pointer<'a> for &'b mut T {
    type Inner = T;

    unsafe fn assignable(self) -> *mut Self::Inner {
        self as *mut T
    }
}

impl<'a, T: ?Sized> Pointer<'a> for *const T {
    type Inner = T;
    unsafe fn assignable(self) -> *mut Self::Inner {
        std::mem::transmute(self)
    }
}

impl<'a, T: ?Sized> Pointer<'a> for *mut T {
    type Inner = T;
    unsafe fn assignable(self) -> *mut Self::Inner {
        self
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

/// Recieves a reference to a pointer-like object and assigns kani::any_modifies to that object.
/// Only for use within function contracts and will not be replaced if the recursive or function stub
/// replace contracts are not used.
#[crate::unstable(feature = "function-contracts", issue = "none", reason = "function-contracts")]
#[rustc_diagnostic_item = "KaniWriteAny"]
#[inline(never)]
#[doc(hidden)]
pub unsafe fn write_any<T: ?Sized>(_pointer: *mut T) {
    // This function should not be reacheable.
    // Users must include `#[kani::recursion]` in any function contracts for recursive functions;
    // otherwise, this might not be properly instantiate. We mark this as unreachable to make
    // sure Kani doesn't report any false positives.
    unreachable!()
}

/// Fill in a slice with kani::any.
/// Intended as a post compilation replacement for write_any
#[crate::unstable(feature = "function-contracts", issue = "none", reason = "function-contracts")]
#[rustc_diagnostic_item = "KaniWriteAnySlice"]
#[inline(always)]
pub unsafe fn write_any_slice<T: Arbitrary>(slice: *mut [T]) {
    (*slice).fill_with(T::any)
}

/// Fill in a pointer with kani::any.
/// Intended as a post compilation replacement for write_any
#[crate::unstable(feature = "function-contracts", issue = "none", reason = "function-contracts")]
#[rustc_diagnostic_item = "KaniWriteAnySlim"]
#[inline(always)]
pub unsafe fn write_any_slim<T: Arbitrary>(pointer: *mut T) {
    ptr::write(pointer, T::any())
}

/// Fill in a str with kani::any.
/// Intended as a post compilation replacement for write_any.
/// Not yet implemented
#[crate::unstable(feature = "function-contracts", issue = "none", reason = "function-contracts")]
#[rustc_diagnostic_item = "KaniWriteAnyStr"]
#[inline(always)]
pub unsafe fn write_any_str(_s: *mut str) {
    //TODO: strings introduce new UB
    //(*s).as_bytes_mut().fill_with(u8::any)
    //TODO: String validation
    unimplemented!("Kani does not support creating arbitrary `str`")
}

/// Function that calls a closure used to implement contracts.
///
/// In contracts, we cannot invoke the generated closures directly, instead, we call register
/// contract. This function is a no-op. However, in the reality, we do want to call the closure,
/// so we swap the register body by this function body.
#[doc(hidden)]
#[allow(dead_code)]
#[rustc_diagnostic_item = "KaniRunContract"]
#[crate::unstable(
    feature = "function-contracts",
    issue = "none",
    reason = "internal function required to run contract closure"
)]
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
