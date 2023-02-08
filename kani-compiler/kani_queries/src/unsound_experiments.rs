// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(feature = "unsound_experiments")]

#[derive(Debug, Clone, Copy, Default)]
pub struct UnsoundExperiments {
    /// Zero initilize variables.
    /// This is useful for experiments to see whether assigning constant values produces better
    /// performance by allowing CBMC to do more constant propegation.
    /// Unfortunatly, it is unsafe to use for production code, since it may unsoundly hide bugs.
    pub zero_init_vars: bool,
}
