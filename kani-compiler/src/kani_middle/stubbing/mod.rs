// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for implementing stubbing.

mod annotations;
mod transform;

pub use annotations::collect_stub_mappings;
pub use transform::*;
