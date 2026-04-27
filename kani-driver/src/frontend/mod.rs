// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Frontend module for handling different output formats and JSON generation
//! This module separates the JSON handling logic from the main verification logic

pub mod json_handler;
pub mod schema_utils;

pub use json_handler::JsonHandler;
pub use schema_utils::*;

#[cfg(test)]
mod tests;
