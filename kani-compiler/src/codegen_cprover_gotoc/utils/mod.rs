// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module provides utils used across Kani

mod debug;
mod names;
#[allow(clippy::module_inception)]
mod utils;

// TODO clean this up

pub use names::*;
pub use utils::*;

pub use debug::init;
