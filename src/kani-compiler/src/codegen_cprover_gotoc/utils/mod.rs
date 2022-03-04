// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module provides utils used across Kani

mod debug;
mod names;
mod utils;

// TODO clean this up

pub use names::*;
pub use utils::*;

pub fn init() {
    debug::init()
}
