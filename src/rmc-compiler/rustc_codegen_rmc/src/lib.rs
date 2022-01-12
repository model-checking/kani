// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(bool_to_option)]
#![feature(crate_visibility_modifier)]
#![feature(extern_types)]
#![feature(in_band_lifetimes)]
#![feature(nll)]
#![recursion_limit = "256"]
#![feature(box_patterns)]
#![feature(once_cell)]
#![feature(rustc_private)]
mod codegen;
mod compiler_interface;
mod context;
mod overrides;
mod utils;

extern crate rustc_arena;
extern crate rustc_ast;
extern crate rustc_attr;
extern crate rustc_codegen_ssa;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_fs_util;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_llvm;
extern crate rustc_metadata;
extern crate rustc_middle;
extern crate rustc_serialize;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;

pub use compiler_interface::GotocCodegenBackend;
pub use context::GotocCtx;
pub use context::VtableCtx;
