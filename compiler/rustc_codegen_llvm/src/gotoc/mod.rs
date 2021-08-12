// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use bitflags::_core::any::Any;
use cbmc::goto_program::symtab_transformer;
use cbmc::goto_program::{Stmt, SymbolTable};
use cbmc::{MachineModel, RoundingMode};
use metadata::*;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::ErrorReported;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::middle::cstore::{EncodedMetadata, MetadataLoaderDyn};
use rustc_middle::mir::mono::{CodegenUnit, MonoItem};
use rustc_middle::ty::query::Providers;
use rustc_middle::ty::{self, TyCtxt};
use rustc_serialize::json::ToJson;
use rustc_session::config::{OutputFilenames, OutputType};
use rustc_session::Session;
use rustc_target::abi::Endian;
use std::lazy::SyncLazy;
use tracing::{debug, warn};

mod assumptions;
mod backend;
mod block;
pub mod cbmc;
mod current_fn;
mod debug;
mod function;
mod hooks;
mod intrinsic;
mod metadata;
mod monomorphize;
mod operand;
mod place;
mod rvalue;
mod statement;
mod static_var;
pub mod stubs;
mod typ;
mod utils;
pub use backend::GotocCodegenBackend;

impl<'tcx> GotocCtx<'tcx> {
    fn should_skip_current_fn(&self) -> bool {
        match self.current_fn().readable_name() {
            // https://github.com/model-checking/rmc/issues/202
            "fmt::ArgumentV1::<'a>::as_usize" => true,
            // https://github.com/model-checking/rmc/issues/204
            name if name.ends_with("__getit") => true,
            // https://github.com/model-checking/rmc/issues/205
            "panic::Location::<'a>::caller" => true,
            // https://github.com/model-checking/rmc/issues/207
            "core::slice::<impl [T]>::split_first" => true,
            // https://github.com/model-checking/rmc/issues/281
            name if name.starts_with("bridge::client") => true,
            // https://github.com/model-checking/rmc/issues/282
            "bridge::closure::Closure::<'a, A, R>::call" => true,
            _ => false,
        }
    }
}
