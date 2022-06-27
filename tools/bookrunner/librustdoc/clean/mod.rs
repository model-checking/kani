// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! This module contains the "cleaned" pieces of the AST, and the functions
//! that clean them.
pub(crate) mod cfg;
//pub(crate) mod inline;
//mod simplify;
pub(crate) mod types;
//pub(crate) mod utils;

pub(crate) use self::types::*;
