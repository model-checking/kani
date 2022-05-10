// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! This module analyzes crates to find call sites that can serve as examples in the documentation.

use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    self as hir,
    intravisit::{self, Visitor},
};
use rustc_macros::{Decodable, Encodable};
use rustc_middle::hir::map::Map;
use rustc_middle::hir::nested_filter;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::{
    def_id::{CrateNum, DefPathHash},
    edition::Edition,
    BytePos, FileName, SourceFile,
};

use std::fs;
use std::path::PathBuf;

#[derive(Encodable, Decodable, Debug, Clone)]
crate struct SyntaxRange {
    crate byte_span: (u32, u32),
    crate line_span: (usize, usize),
}

impl SyntaxRange {
    fn new(span: rustc_span::Span, file: &SourceFile) -> Self {
        let get_pos = |bytepos: BytePos| file.original_relative_byte_pos(bytepos).0;
        let get_line = |bytepos: BytePos| file.lookup_line(bytepos).unwrap();

        SyntaxRange {
            byte_span: (get_pos(span.lo()), get_pos(span.hi())),
            line_span: (get_line(span.lo()), get_line(span.hi())),
        }
    }
}

#[derive(Encodable, Decodable, Debug, Clone)]
crate struct CallLocation {
    crate call_expr: SyntaxRange,
    crate enclosing_item: SyntaxRange,
}

impl CallLocation {
    fn new(
        expr_span: rustc_span::Span,
        enclosing_item_span: rustc_span::Span,
        source_file: &SourceFile,
    ) -> Self {
        CallLocation {
            call_expr: SyntaxRange::new(expr_span, source_file),
            enclosing_item: SyntaxRange::new(enclosing_item_span, source_file),
        }
    }
}

#[derive(Encodable, Decodable, Debug, Clone)]
crate struct CallData {
    crate locations: Vec<CallLocation>,
    crate url: String,
    crate display_name: String,
    crate edition: Edition,
}

crate type FnCallLocations = FxHashMap<PathBuf, CallData>;
crate type AllCallLocations = FxHashMap<DefPathHash, FnCallLocations>;
