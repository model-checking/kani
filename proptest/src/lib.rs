//-
// Copyright 2017, 2018 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Proptest Reference Documentation
//!
//! This is the reference documentation for the proptest API.
//!
//! For documentation on how to get started with proptest and general usage
//! advice, please refer to the [Proptest Book](https://altsysrq.github.io/proptest-book/intro.html).

#![forbid(future_incompatible)]
#![deny(missing_docs, bare_trait_objects)]
#![no_std]
#![cfg_attr(feature = "cargo-clippy", allow(
    doc_markdown,
    // We have a lot of these lints for associated types... And we don't care.
    type_complexity
))]
#![cfg_attr(
    feature = "unstable",
    feature(
        allocator_api,
        try_trait,
        generator_trait,
        never_type,
        try_reserve
    )
)]
#![cfg_attr(all(feature = "std", feature = "unstable"), feature(ip))]
#![cfg_attr(
    all(feature = "alloc", not(feature = "std")),
    feature(core_intrinsics)
)]

// std_facade is used in a few macros, so it needs to be public.
#[macro_use]
#[doc(hidden)]
pub mod std_facade;

#[cfg(any(feature = "std", test))]
#[macro_use]
extern crate std;

#[cfg(all(feature = "alloc", not(feature = "std")))]
#[macro_use]
extern crate alloc;

#[cfg(feature = "frunk")]
#[macro_use]
extern crate frunk_core;

#[cfg(feature = "frunk")]
#[macro_use]
mod product_frunk;

#[cfg(not(feature = "frunk"))]
#[macro_use]
mod product_tuple;

#[macro_use]
extern crate bitflags;
#[cfg(feature = "bit-set")]
extern crate bit_set;

#[cfg(feature = "std")]
#[macro_use]
extern crate lazy_static;

// Only required for the string module.
#[cfg(feature = "std")]
#[macro_use]
extern crate quick_error;

#[cfg(feature = "fork")]
#[macro_use]
extern crate rusty_fork;

#[macro_use]
mod macros;

#[doc(hidden)]
#[macro_use]
pub mod sugar;

pub mod arbitrary;
pub mod array;
pub mod bits;
pub mod bool;
pub mod char;
pub mod collection;
pub mod num;
pub mod strategy;
pub mod test_runner;
pub mod tuple;

pub mod option;
pub mod result;
pub mod sample;
#[cfg(feature = "std")]
pub mod string;

pub mod prelude;
