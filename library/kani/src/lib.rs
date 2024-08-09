// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Required so we can use kani_macros attributes.
#![feature(register_tool)]
#![register_tool(kanitool)]
// Used for rustc_diagnostic_item.
// Note: We could use a kanitool attribute instead.
#![feature(rustc_attrs)]
// Used to model simd.
#![feature(repr_simd)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
// Features used for tests only.
#![cfg_attr(test, feature(core_intrinsics, portable_simd))]
// Required for `rustc_diagnostic_item` and `core_intrinsics`
#![allow(internal_features)]
// Required for implementing memory predicates.
#![feature(ptr_metadata)]
#![feature(f16)]
#![feature(f128)]

// Allow us to use `kani::` to access crate features.
extern crate self as kani;

pub mod arbitrary;
#[cfg(feature = "concrete_playback")]
mod concrete_playback;
pub mod futures;
pub mod invariant;
pub mod shadow;
pub mod slice;
pub mod vec;

mod mem_init;
mod models;

#[cfg(feature = "concrete_playback")]
pub use concrete_playback::concrete_playback_run;
pub use invariant::Invariant;

#[cfg(not(feature = "concrete_playback"))]
/// NOP `concrete_playback` for type checking during verification mode.
pub fn concrete_playback_run<F: Fn()>(_: Vec<Vec<u8>>, _: F) {
    unreachable!("Concrete playback does not work during verification")
}

pub use futures::{block_on, block_on_with_spawn, spawn, yield_now, RoundRobin};

// Kani proc macros must be in a separate crate
pub use kani_macros::*;

// Declare common Kani API such as assume, assert
kani_core::kani_lib!(kani);

// Used to bind `core::assert` to a different name to avoid possible name conflicts if a
// crate uses `extern crate std as core`. See
// https://github.com/model-checking/kani/issues/1949 and https://github.com/model-checking/kani/issues/2187
#[doc(hidden)]
#[cfg(not(feature = "concrete_playback"))]
pub use core::assert as __kani__workaround_core_assert;

#[macro_export]
macro_rules! cover {
    () => {
        kani::cover(true, "cover location");
    };
    ($cond:expr $(,)?) => {
        kani::cover($cond, concat!("cover condition: ", stringify!($cond)));
    };
    ($cond:expr, $msg:literal) => {
        kani::cover($cond, $msg);
    };
}

#[macro_export]
macro_rules! cover_or_fail {
    () => {
        kani::cover_or_fail(true, "cover location");
    };
    ($cond:expr $(,)?) => {
        kani::cover_or_fail($cond, concat!("cover condition: ", stringify!($cond)));
    };
    ($cond:expr, $msg:literal) => {
        kani::cover_or_fail($cond, $msg);
    };
}

/// `implies!(premise => conclusion)` means that if the `premise` is true, so
/// must be the `conclusion`.
///
/// This simply expands to `!premise || conclusion` and is intended to make checks more readable,
/// as the concept of an implication is more natural to think about than its expansion.
#[macro_export]
macro_rules! implies {
    ($premise:expr => $conclusion:expr) => {
        !($premise) || ($conclusion)
    };
}

pub(crate) use kani_macros::unstable_feature as unstable;

pub mod contracts;
