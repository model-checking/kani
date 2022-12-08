// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code that are backend agnostic. For example, MIR analysis
//! and transformations.
pub mod attributes;
pub mod coercion;
pub mod provide;
pub mod reachability;
pub mod stubbing;
