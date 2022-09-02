//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

//! Defines macros for product type creation, extraction, and the type signature
//! itself. This version uses tuples. This mechanism is used to be very
//! loosely coupled with `frunk_core` so that only `lib.rs` has to be changed
//! in the event that Rust gets tuple-variadic generics.

#![allow(unused_macros)] // Merge in progress. See #1608 for details

macro_rules! product_type {
    ($factor: ty) => {
        ($factor,)
    };
    ($($factor: ty),*) => {
        ( $( $factor, )* )
    };
    ($($factor: ty),*,) => {
        ( $( $factor, )* )
    };
}

macro_rules! product_pack {
    ($factor: expr) => {
        ($factor,)
    };
    ($($factor: expr),*) => {
        ( $( $factor ),* )
    };
    ($($factor: expr),*,) => {
        ( $( $factor ),* )
    };
}

macro_rules! product_unpack {
    ($factor: pat) => {
        ($factor,)
    };
    ($($factor: pat),*) => {
        ( $( $factor ),* )
    };
    ($($factor: pat),*,) => {
        ( $( $factor ),* )
    };
}
