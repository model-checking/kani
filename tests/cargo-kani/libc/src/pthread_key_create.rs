// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Check that Kani doesn't crash if it invokes calls to pthread_key_create.
//!
//! The declaration of `pthread_key_create` in the libc crate differs in type from the C one.
//! The rust libc crate uses `Option<unsafe extern "C" fn(*mut u8)>` to represent the optional
//! destructor, while the C one takes a function pointer.
//!
//! See <https://github.com/model-checking/kani/issues/1781> for more details.
//!
//! Until we add full support to C-FFI, functions that are not explicitly declared in the Kani
//! compiler setup will be codegen as unsupported.
//!
//! This test ensures that a harness only fails during verification if the call is reachable.
//!
//! TODO: Add the following tests:
//!    - Calling via fn pointer.
//!    - Calling rust extern "C" functions directly + via pointer.
//!    - Calling variadic C function.
//!
use libc;

#[kani::proof]
pub fn check_create() {
    let mut key = 0;
    let _res = unsafe { libc::pthread_key_create(&mut key, None) };
}
