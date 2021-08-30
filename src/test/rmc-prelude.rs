// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Passing a --use-abs flag to rmc allows rmc users to replace out parts of the
// standard library with simpler, verification-friendly stubs. The prelude chooses
// a specific abstraction depending on the --abs-type flag given to rmc.
// This is currently only implemented for the Vec abstractionbut a PoC code for
// HashSet is given too.
//
// Eventually we wouldd want to move to a more stable method of performing
// stubbing.
// Tracking issue: https://github.com/model-checking/rmc/issues/455
//
// The default option "std" uses the standard library implementation.
// rmc uses the fine-grained, std compatible but verification-friendly implementation
// C-FFI and NoBackVec are more experimental abstractions.

#[cfg(not(use_abs))]
use std::vec::Vec;

#[cfg(use_abs)]
mod stub;

#[cfg(use_abs)]
#[cfg(abs_type = "no-back")]
use stub::noback_vec::Vec;

#[cfg(use_abs)]
#[cfg(abs_type = "c-ffi")]
use stub::c_vec::Vec;

#[cfg(use_abs)]
#[cfg(abs_type = "rmc")]
use stub::rmc_vec::Vec;

#[cfg(use_abs)]
#[cfg(abs_type = "rmc")]
use stub::hashset::HashSet;

fn __VERIFIER_assume(cond: bool) {
    unimplemented!()
}

fn __VERIFIER_expect_fail(cond: bool, message: &str) {
    unimplemented!()
}

fn __nondet<T>() -> T {
    unimplemented!()
}
