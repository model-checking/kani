// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(feature = "unsound_experiments")]

#[derive(Debug, Default)]
pub struct UnsoundExperiments {
    pub zero_init_vars: bool,
}
