// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use super::cbmc::goto_program::symtab_transformer;
use super::cbmc::goto_program::SymbolTable;
use super::cbmc::{MachineModel, RoundingMode};
use super::metadata::*;
use super::monomorphize;
use bitflags::_core::any::Any;
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

// #[derive(RustcEncodable, RustcDecodable)]
pub struct GotocCodegenResult {
    pub symtab: SymbolTable,
    pub crate_name: rustc_span::Symbol,
}

#[derive(Clone)]
pub struct GotocCodegenBackend();

impl GotocCodegenBackend {
    pub fn new() -> Box<dyn CodegenBackend> {
        Box::new(GotocCodegenBackend())
    }
}

impl CodegenBackend for GotocCodegenBackend {
    fn metadata_loader(&self) -> Box<MetadataLoaderDyn> {
        Box::new(rustc_codegen_ssa::back::metadata::DefaultMetadataLoader)
    }

    fn provide(&self, providers: &mut Providers) {
        monomorphize::partitioning::provide(providers);
    }

    fn provide_extern(&self, _providers: &mut ty::query::Providers) {}

    fn codegen_crate<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        _metadata: EncodedMetadata,
        _need_metadata_module: bool,
    ) -> Box<dyn Any> {
        use rustc_hir::def_id::LOCAL_CRATE;

        // Install panic hook
        SyncLazy::force(&super::debug::DEFAULT_HOOK); // Install ice hook

        let codegen_units: &'tcx [CodegenUnit<'_>] = tcx.collect_and_partition_mono_items(()).1;
        let mm = machine_model_from_session(&tcx.sess);
        let mut c = GotocCtx::new(tcx, SymbolTable::new(mm));

        // we first declare all functions
        for cgu in codegen_units {
            let items = cgu.items_in_deterministic_order(tcx);
            for (item, _) in items {
                match item {
                    MonoItem::Fn(instance) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.declare_function(instance),
                            format!("declare_function: {}", c.readable_instance_name(instance)),
                        );
                    }
                    MonoItem::Static(def_id) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.declare_static(def_id, item),
                            format!("declare_static: {:?}", def_id),
                        );
                    }
                    MonoItem::GlobalAsm(_) => {
                        warn!(
                            "Crate {} contains global ASM, which is not handled by RMC",
                            c.crate_name()
                        );
                    }
                }
            }
        }

        // then we move on to codegen
        for cgu in codegen_units {
            let items = cgu.items_in_deterministic_order(tcx);
            for (item, _) in items {
                match item {
                    MonoItem::Fn(instance) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.codegen_function(instance),
                            format!("codegen_function: {}", c.readable_instance_name(instance)),
                        );
                    }
                    MonoItem::Static(def_id) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.codegen_static(def_id, item),
                            format!("codegen_static: {:?}", def_id),
                        );
                    }
                    MonoItem::GlobalAsm(_) => {} // We have already warned above
                }
            }
        }

        // perform post-processing symbol table passes
        let symbol_table = symtab_transformer::do_passes(
            c.symbol_table,
            &tcx.sess.opts.debugging_opts.symbol_table_passes,
        );

        Box::new(GotocCodegenResult {
            symtab: symbol_table,
            crate_name: tcx.crate_name(LOCAL_CRATE) as rustc_span::Symbol,
        })
    }

    fn join_codegen(
        &self,
        ongoing_codegen: Box<dyn Any>,
        _sess: &Session,
    ) -> Result<(Box<dyn Any>, FxHashMap<WorkProductId, WorkProduct>), ErrorReported> {
        Ok((ongoing_codegen, FxHashMap::default()))
    }

    fn link(
        &self,
        _sess: &Session,
        codegen_results: Box<dyn Any>,
        outputs: &OutputFilenames,
    ) -> Result<(), ErrorReported> {
        use std::io::Write;

        let result = codegen_results
            .downcast::<GotocCodegenResult>()
            .expect("in link: codegen_results is not a GotocCodegenResult");
        let symtab = result.symtab;
        let irep_symtab = symtab.to_irep();
        let json = irep_symtab.to_json();
        let pretty_json = json.pretty();

        let output_name = outputs.path(OutputType::Object).with_extension("json");
        debug!("output to {:?}", output_name);
        let mut out_file = ::std::fs::File::create(output_name).unwrap();
        write!(out_file, "{}", pretty_json.to_string()).unwrap();

        Ok(())
    }
}

fn machine_model_from_session(sess: &Session) -> MachineModel {
    // TODO: Hardcoded values from from the ones currently used in env.rs
    // We may wish to get more of them from the session.
    let alignment = sess.target.options.min_global_align.unwrap_or(1);
    let architecture = &sess.target.arch;
    let bool_width = 8;
    let char_is_unsigned = false;
    let char_width = 8;
    let double_width = 64;
    let float_width = 32;
    let int_width = 32;
    let is_big_endian = match sess.target.options.endian {
        Endian::Little => false,
        Endian::Big => true,
    };
    let long_double_width = 128;
    let long_int_width = 64;
    let long_long_int_width = 64;
    let memory_operand_size = 4;
    let null_is_zero = true;
    let pointer_width = sess.target.pointer_width.into();
    let short_int_width = 16;
    let single_width = 32;
    let wchar_t_is_unsigned = false;
    let wchar_t_width = 32;
    let word_size = 32;
    let rounding_mode = RoundingMode::ToNearest;

    MachineModel::new(
        alignment,
        architecture,
        bool_width,
        char_is_unsigned,
        char_width,
        double_width,
        float_width,
        int_width,
        is_big_endian,
        long_double_width,
        long_int_width,
        long_long_int_width,
        memory_operand_size,
        null_is_zero,
        pointer_width,
        rounding_mode,
        short_int_width,
        single_width,
        wchar_t_is_unsigned,
        wchar_t_width,
        word_size,
    )
}
