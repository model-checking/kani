// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
#![doc(
    html_root_url = "https://doc.rust-lang.org/nightly/",
    html_playground_url = "https://play.rust-lang.org/"
)]
#![feature(rustc_private)]
#![feature(array_methods)]
#![feature(assert_matches)]
#![feature(box_patterns)]
#![feature(control_flow_enum)]
#![feature(test)]
#![feature(never_type)]
#![feature(type_ascription)]
#![feature(iter_intersperse)]
#![recursion_limit = "256"]
#![warn(rustc::internal)]
#![allow(clippy::collapsible_if, clippy::collapsible_else_if, clippy::arc_with_non_send_sync)]

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
extern crate test;

pub mod doctest;
// used by the error-index generator, so it needs to be public
pub mod html;
