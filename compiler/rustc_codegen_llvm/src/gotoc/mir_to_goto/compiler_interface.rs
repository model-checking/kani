// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use crate::gotoc::cbmc::goto_program::symtab_transformer;
use crate::gotoc::cbmc::goto_program::SymbolTable;
use crate::gotoc::mir_to_goto::monomorphize;
use crate::gotoc::mir_to_goto::GotocCtx;
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

        super::utils::init();

        let codegen_units: &'tcx [CodegenUnit<'_>] = tcx.collect_and_partition_mono_items(()).1;
        let mut c = GotocCtx::new(tcx);

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
