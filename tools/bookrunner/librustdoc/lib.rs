#![doc(
    html_root_url = "https://doc.rust-lang.org/nightly/",
    html_playground_url = "https://play.rust-lang.org/"
)]
#![feature(rustc_private)]
#![feature(array_methods)]
#![feature(assert_matches)]
#![feature(box_patterns)]
#![feature(control_flow_enum)]
#![feature(box_syntax)]
#![feature(let_else)]
#![feature(nll)]
#![feature(test)]
#![feature(crate_visibility_modifier)]
#![feature(never_type)]
#![feature(once_cell)]
#![feature(type_ascription)]
#![feature(iter_intersperse)]
#![recursion_limit = "256"]
#![warn(rustc::internal)]
#![allow(clippy::collapsible_if, clippy::collapsible_else_if)]

#[macro_use]
extern crate tracing;

// N.B. these need `extern crate` even in 2018 edition
// because they're loaded implicitly from the sysroot.
// The reason they're loaded from the sysroot is because
// the rustdoc artifacts aren't stored in rustc's cargo target directory.
// So if `rustc` was specified in Cargo.toml, this would spuriously rebuild crates.
//
// Dependencies listed in Cargo.toml do not need `extern crate`.

extern crate rustc_ast;
extern crate rustc_ast_lowering;
extern crate rustc_ast_pretty;
extern crate rustc_attr;
extern crate rustc_const_eval;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_expand;
extern crate rustc_feature;
extern crate rustc_hir;
extern crate rustc_hir_pretty;
extern crate rustc_index;
extern crate rustc_infer;
extern crate rustc_interface;
extern crate rustc_lexer;
extern crate rustc_lint;
extern crate rustc_lint_defs;
extern crate rustc_macros;
extern crate rustc_metadata;
extern crate rustc_middle;
extern crate rustc_parse;
extern crate rustc_passes;
extern crate rustc_resolve;
extern crate rustc_serialize;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_trait_selection;
extern crate rustc_typeck;
extern crate test;

// See docs in https://github.com/rust-lang/rust/blob/master/compiler/rustc/src/main.rs
// about jemalloc.
#[cfg(feature = "jemalloc")]
extern crate tikv_jemalloc_sys;
#[cfg(feature = "jemalloc")]
use tikv_jemalloc_sys as jemalloc_sys;

use std::default::Default;
use std::env::{self, VarError};
use std::io;
use std::process;

use rustc_driver::{abort_on_err, describe_lints};
use rustc_errors::ErrorReported;
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::{make_crate_type_option, ErrorOutputType, RustcOptGroup};
use rustc_session::getopts;
use rustc_session::{early_error, early_warn};

use crate::clean::utils::DOC_RUST_LANG_ORG_CHANNEL;
use crate::passes::collect_intra_doc_links;

/// A macro to create a FxHashMap.
///
/// Example:
///
/// ```
/// let letters = map!{"a" => "b", "c" => "d"};
/// ```
///
/// Trailing commas are allowed.
/// Commas between elements are required (even if the expression is a block).
macro_rules! map {
    ($( $key: expr => $val: expr ),* $(,)*) => {{
        let mut map = ::rustc_data_structures::fx::FxHashMap::default();
        $( map.insert($key, $val); )*
        map
    }}
}

mod clean;
mod config;
mod core;
mod docfs;
pub mod doctest;
mod error;
mod externalfiles;
mod fold;
mod formats;
// used by the error-index generator, so it needs to be public
pub mod html;
mod json;
crate mod lint;
mod markdown;
mod passes;
mod scrape_examples;
mod theme;
mod visit;
mod visit_ast;
mod visit_lib;
