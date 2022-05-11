// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! This module analyzes crates to find call sites that can serve as examples in the documentation.

use rustc_macros::{Decodable, Encodable};
use rustc_span::edition::Edition;

#[derive(Encodable, Decodable, Debug, Clone)]
crate struct SyntaxRange {
    crate byte_span: (u32, u32),
    crate line_span: (usize, usize),
}

#[derive(Encodable, Decodable, Debug, Clone)]
crate struct CallLocation {
    crate call_expr: SyntaxRange,
    crate enclosing_item: SyntaxRange,
}

#[derive(Encodable, Decodable, Debug, Clone)]
crate struct CallData {
    crate locations: Vec<CallLocation>,
    crate url: String,
    crate display_name: String,
    crate edition: Edition,
}
