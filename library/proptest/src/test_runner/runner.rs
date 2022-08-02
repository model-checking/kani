//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
// Modifications Copyright Kani Contributors
// See GitHub history for details

//! This file implements a Kani-specific test runner that mirrors the
//! one in proptest. In contrast to the original, this test runner
//! will run once but with symbolic inputs.

use crate::strategy::{
    Strategy,
    ValueTree,
};
use crate::test_runner::Config;

/// Fake test runner that keeps no state.
pub struct TestRunner {}

impl TestRunner {

    /// Creates new
    pub fn new(_: Config) -> Self { Self {} }

    /// default test runner.
    pub fn default() -> Self { Self {} }

    /// Run the test function with a Kani symbolic value given a test function that takes that type.
    pub fn run_kani<S: Strategy>(strategy: S, test_fn: impl Fn(S::Value)) {
        let mut runner = Self::new(Config::default());
        let tree = strategy.new_tree(&mut runner).unwrap();
        test_fn(tree.current());
    }

}
