// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use crate::overrides::skip_monomorphize;
use crate::GotocCtx;
use bitflags::_core::any::Any;
use cbmc::goto_program::symtab_transformer;
use cbmc::goto_program::SymbolTable;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::ErrorReported;
use rustc_metadata::EncodedMetadata;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::mir::mono::{CodegenUnit, MonoItem};
use rustc_middle::ty::query::Providers;
use rustc_middle::ty::{self, TyCtxt};
use rustc_serialize::json::ToJson;
use rustc_session::config::{OutputFilenames, OutputType};
use rustc_session::cstore::MetadataLoaderDyn;
use rustc_session::Session;
use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::path::PathBuf;
use tracing::{debug, warn};

// #[derive(RustcEncodable, RustcDecodable)]
pub struct GotocCodegenResult {
    pub type_map: BTreeMap<String, String>,
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
        providers.skip_monomorphize = skip_monomorphize;
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
                            instance.def_id(),
                        );
                    }
                    MonoItem::Static(def_id) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.declare_static(def_id, item),
                            format!("declare_static: {:?}", def_id),
                            def_id,
                        );
                    }
                    MonoItem::GlobalAsm(_) => {
                        warn!(
                            "Crate {} contains global ASM, which is not handled by RMC",
                            c.short_crate_name()
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
                            format!(
                                "codegen_function: {}\n{}",
                                c.readable_instance_name(instance),
                                c.symbol_name(instance)
                            ),
                            instance.def_id(),
                        );
                    }
                    MonoItem::Static(def_id) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.codegen_static(def_id, item),
                            format!("codegen_static: {:?}", def_id),
                            def_id,
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

        let type_map = BTreeMap::from_iter(c.type_map.into_iter().map(|(k, v)| (k, v.to_string())));

        Box::new(GotocCodegenResult {
            type_map,
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

        // "path.o"
        let base_filename = outputs.path(OutputType::Object);

        let symtab_filename = base_filename.with_extension("symtab.json");
        debug!("output to {:?}", symtab_filename);
        let mut out_file = ::std::fs::File::create(&symtab_filename).unwrap();
        write!(out_file, "{}", result.symtab.to_irep().to_json().pretty().to_string()).unwrap();

        let type_map_filename = base_filename.with_extension("type_map.json");
        debug!("type_map to {:?}", type_map_filename);
        let mut out_file = ::std::fs::File::create(&type_map_filename).unwrap();
        write!(out_file, "{}", result.type_map.to_json().pretty().to_string()).unwrap();

        Ok(())
    }
}
