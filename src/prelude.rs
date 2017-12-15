//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Re-exports the most commonly-needed APIs of proptest.
//!
//! This module is intended to be wildcard-imported, i.e.,
//! `use proptest::prelude::*;`. Note that it re-exports the whole crate itself
//! under the name `prop`, so you don't need a separate `use proptest;` line.

pub use strategy::{BoxedStrategy, Just, SBoxedStrategy, Strategy};
pub use test_runner::Config as ProptestConfig;
pub use test_runner::TestCaseError;
pub use test_runner::fail_case;
pub use test_runner::reject_case;

/// Re-exports the entire public API of proptest so that an import of `prelude`
/// allows simply writing, for example, `prop::num::i32::ANY` rather than
/// `proptest::num::i32::ANY` plus a separate `use proptest;`.
pub mod prop {
    pub use test_runner;
    pub use strategy;
    pub use bool;
    pub use num;
    pub use bits;
    pub use tuple;
    pub use array;
    pub use collection;
    pub use char;
    pub use string;
    pub use option;
    pub use result;
    pub use sample;
}
