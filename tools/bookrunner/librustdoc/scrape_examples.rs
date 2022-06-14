// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! This module analyzes crates to find call sites that can serve as examples in the documentation.

use rustc_macros::{Decodable, Encodable};
use rustc_span::edition::Edition;

#[derive(Encodable, Decodable, Debug, Clone)]
pub(crate) struct SyntaxRange {
    pub(crate) byte_span: (u32, u32),
    pub(crate) line_span: (usize, usize),
}

#[derive(Encodable, Decodable, Debug, Clone)]
pub(crate) struct CallLocation {
    pub(crate) call_expr: SyntaxRange,
    pub(crate) enclosing_item: SyntaxRange,
}

#[derive(Encodable, Decodable, Debug, Clone)]
pub(crate) struct CallData {
    pub(crate) locations: Vec<CallLocation>,
    pub(crate) url: String,
    pub(crate) display_name: String,
    pub(crate) edition: Edition,
}
