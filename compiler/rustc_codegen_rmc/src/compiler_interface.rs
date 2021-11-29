// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use crate::context::metadata::RmcMetadata;
use crate::GotocCtx;
use bitflags::_core::any::Any;
use cbmc::goto_program::symtab_transformer;
use cbmc::goto_program::SymbolTable;
use cbmc::InternedString;
use rmc_restrictions::VtableCtxResults;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::ErrorReported;
use rustc_metadata::EncodedMetadata;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::mir::mono::{CodegenUnit, MonoItem};
use rustc_middle::ty::query::Providers;
use rustc_middle::ty::{self, TyCtxt};
use rustc_session::config::{OutputFilenames, OutputType};
use rustc_session::cstore::MetadataLoaderDyn;
use rustc_session::Session;
use std::collections::BTreeMap;
use std::io::BufWriter;
use std::iter::FromIterator;
use std::path::PathBuf;
use tracing::{debug, warn};

// #[derive(RustcEncodable, RustcDecodable)]
pub struct GotocCodegenResult {
    pub crate_name: rustc_span::Symbol,
    pub metadata: RmcMetadata,
    pub symtab: SymbolTable,
    pub type_map: BTreeMap<InternedString, InternedString>,
    pub vtable_restrictions: Option<VtableCtxResults>,
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

    fn provide(&self, _providers: &mut Providers) {}

    fn provide_extern(&self, _providers: &mut ty::query::ExternProviders) {}

    fn codegen_crate<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        _metadata: EncodedMetadata,
        _need_metadata_module: bool,
    ) -> Box<dyn Any> {
        use rustc_hir::def_id::LOCAL_CRATE;

        if !tcx.sess.overflow_checks() {
            tcx.sess.err("RMC requires overflow checks in order to provide a sound analysis.")
        }

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
        let symtab = symtab_transformer::do_passes(
            c.symbol_table,
            &tcx.sess.opts.debugging_opts.symbol_table_passes,
        );

        // Map MIR types to GotoC types
        let type_map =
            BTreeMap::from_iter(c.type_map.into_iter().map(|(k, v)| (k, v.to_string().into())));

        // Get the vtable function pointer restrictions if requested
        let vtable_restrictions = if c.vtable_ctx.emit_vtable_restrictions {
            Some(c.vtable_ctx.get_virtual_function_restrictions())
        } else {
            None
        };

        let metadata = RmcMetadata { proof_harnesses: c.proof_harnesses };

        Box::new(GotocCodegenResult {
            crate_name: tcx.crate_name(LOCAL_CRATE) as rustc_span::Symbol,
            metadata,
            symtab,
            type_map,
            vtable_restrictions,
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
        sess: &Session,
        codegen_results: Box<dyn Any>,
        outputs: &OutputFilenames,
    ) -> Result<(), ErrorReported> {
        let result = codegen_results
            .downcast::<GotocCodegenResult>()
            .expect("in link: codegen_results is not a GotocCodegenResult");

        // No output should be generated if user selected no_codegen.
        if !sess.opts.debugging_opts.no_codegen && sess.opts.output_types.should_codegen() {
            // "path.o"
            let base_filename = outputs.path(OutputType::Object);
            write_file(&base_filename, "symtab.json", &result.symtab);
            write_file(&base_filename, "type_map.json", &result.type_map);
            write_file(&base_filename, "rmc-metadata.json", &result.metadata);
            // If they exist, write out vtable virtual call function pointer restrictions
            if let Some(restrictions) = result.vtable_restrictions {
                write_file(&base_filename, "restrictions.json", &restrictions);
            }
        }

        Ok(())
    }
}

fn write_file<T>(base_filename: &PathBuf, extension: &str, source: &T)
where
    T: serde::Serialize,
{
    let filename = base_filename.with_extension(extension);
    debug!("output to {:?}", filename);
    let out_file = ::std::fs::File::create(&filename).unwrap();
    let writer = BufWriter::new(out_file);
    serde_json::to_writer(writer, &source).unwrap();
}
