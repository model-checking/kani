// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A `#[rustc_comptime]` function's body is a const context, which `optimized_mir` refuses to
//! serve while `is_mir_available` still reports `true`. The const-block precondition search added
//! in #4820 fetched it that way and aborted the whole run on any crate declaring one -- `core`
//! declares 27: https://github.com/model-checking/kani/issues/4839
//!
//! The three shapes below are the ones `core` has, and the search has to reach the third: a
//! const-block precondition in a const-context body still has to be found, not skipped.

#![feature(intrinsics, rustc_attrs)]

// Body-less, declared exactly as `core::intrinsics::size_of` is.
#[rustc_nounwind]
#[rustc_intrinsic]
#[rustc_comptime]
pub fn size_of<T>() -> usize;

// With a body, as `core::any::TypeId::trait_info_of` is.
#[rustc_comptime]
pub fn with_body(x: u32) -> u32 {
    x + 1
}

// With a body, and a const-block precondition on a const generic parameter.
#[rustc_comptime]
pub fn guarded<const N: usize>() -> usize {
    const { assert!(N >= 4) };
    N
}

// A plain `const fn` with the same precondition: not a const *context*, so it is served by
// `optimized_mir`. Kept here so that a future change to how the body is fetched cannot lose the
// detection for the shape `core::escape` uses, which is why #4820 exists.
pub const fn guarded_const_fn<const N: usize>(x: u8) -> u8 {
    const { assert!(N >= 4) };
    x
}

pub fn plain(x: u8) -> u8 {
    x
}
