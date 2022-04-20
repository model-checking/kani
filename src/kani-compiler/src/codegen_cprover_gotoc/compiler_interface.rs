// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use crate::codegen_cprover_gotoc::GotocCtx;
use bitflags::_core::any::Any;
use cbmc::goto_program::symtab_transformer;
use cbmc::InternedString;
use kani_metadata::KaniMetadata;
use kani_queries::{QueryDb, UserInput};
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_codegen_ssa::{CodegenResults, CrateInfo};
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::ErrorGuaranteed;
use rustc_metadata::EncodedMetadata;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::mir::mono::{CodegenUnit, MonoItem};
use rustc_middle::ty::query::Providers;
use rustc_middle::ty::{self, TyCtxt};
use rustc_session::config::{OutputFilenames, OutputType};
use rustc_session::cstore::MetadataLoaderDyn;
use rustc_session::Session;
use rustc_target::abi::Endian;
use rustc_target::spec::PanicStrategy;
use std::collections::BTreeMap;
use std::io::BufWriter;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::rc::Rc;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct GotocCodegenBackend {
    queries: Rc<QueryDb>,
}

impl GotocCodegenBackend {
    pub fn new(queries: &Rc<QueryDb>) -> Box<dyn CodegenBackend> {
        Box::new(GotocCodegenBackend { queries: Rc::clone(&queries) })
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
        rustc_metadata: EncodedMetadata,
        need_metadata_module: bool,
    ) -> Box<dyn Any> {
        super::utils::init();

        check_target(&tcx.sess);
        check_options(&tcx.sess, need_metadata_module);

        let codegen_units: &'tcx [CodegenUnit<'_>] = tcx.collect_and_partition_mono_items(()).1;
        let mut c = GotocCtx::new(tcx, self.queries.clone());

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
                            "Crate {} contains global ASM, which is not handled by Kani",
                            c.short_crate_name()
                        );
                    }
                }
            }
        }

        // We currently don't model global ASM:
        // https://github.com/model-checking/kani/issues/316
        // so if crate has global ASM, leave all functions in this crate
        // undefined so that calling any of them would hit an assert false
        if !crate_has_global_asm(codegen_units, tcx) {
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
        }

        // perform post-processing symbol table passes
        let passes = self.queries.get_symbol_table_passes();
        let symtab = symtab_transformer::do_passes(c.symbol_table, &passes);

        // Map MIR types to GotoC types
        let type_map: BTreeMap<InternedString, InternedString> =
            BTreeMap::from_iter(c.type_map.into_iter().map(|(k, v)| (k, v.to_string().into())));

        // Get the vtable function pointer restrictions if requested
        let vtable_restrictions = if c.vtable_ctx.emit_vtable_restrictions {
            Some(c.vtable_ctx.get_virtual_function_restrictions())
        } else {
            None
        };

        let metadata = KaniMetadata { proof_harnesses: c.proof_harnesses };

        // No output should be generated if user selected no_codegen.
        if !tcx.sess.opts.debugging_opts.no_codegen && tcx.sess.opts.output_types.should_codegen() {
            let outputs = tcx.output_filenames(());
            let base_filename = outputs.output_path(OutputType::Object);
            let pretty = self.queries.get_output_pretty_json();
            write_file(&base_filename, "symtab.json", &symtab, pretty);
            write_file(&base_filename, "type_map.json", &type_map, pretty);
            write_file(&base_filename, "kani-metadata.json", &metadata, pretty);
            // If they exist, write out vtable virtual call function pointer restrictions
            if let Some(restrictions) = vtable_restrictions {
                write_file(&base_filename, "restrictions.json", &restrictions, pretty);
            }
        }

        let work_products = FxHashMap::<WorkProductId, WorkProduct>::default();
        Box::new((
            CodegenResults {
                modules: vec![],
                allocator_module: None,
                metadata_module: None,
                metadata: rustc_metadata,
                crate_info: CrateInfo::new(tcx, symtab.machine_model().architecture().to_string()),
            },
            work_products,
        ))
    }

    fn join_codegen(
        &self,
        ongoing_codegen: Box<dyn Any>,
        _sess: &Session,
        _filenames: &OutputFilenames,
    ) -> Result<(CodegenResults, FxHashMap<WorkProductId, WorkProduct>), ErrorGuaranteed> {
        Ok(*ongoing_codegen
            .downcast::<(CodegenResults, FxHashMap<WorkProductId, WorkProduct>)>()
            .unwrap())
    }

    fn link(
        &self,
        _sess: &Session,
        _codegen_results: CodegenResults,
        _outputs: &OutputFilenames,
    ) -> Result<(), ErrorGuaranteed> {
        Ok(())
    }
}

fn check_target(session: &Session) {
    // The requirement below is needed to build a valid CBMC machine model
    // in function `machine_model_from_session` from
    // src/kani-compiler/src/codegen_cprover_gotoc/context/goto_ctx.rs
    let is_linux_target = session.target.llvm_target == "x86_64-unknown-linux-gnu";
    // Comparison with `x86_64-apple-darwin` does not work well because the LLVM
    // target may become `x86_64-apple-macosx10.7.0` (or similar) and fail
    let is_darwin_target = session.target.llvm_target.starts_with("x86_64-apple-");

    if !is_linux_target && !is_darwin_target {
        let err_msg = format!(
            "Kani requires the target platform to be `x86_64-unknown-linux-gnu` or `x86_64-apple-darwin`, but it is {}",
            &session.target.llvm_target
        );
        session.err(&err_msg);
    }

    session.abort_if_errors();
}

fn check_options(session: &Session, need_metadata_module: bool) {
    // The requirements for `min_global_align` and `endian` are needed to build
    // a valid CBMC machine model in function `machine_model_from_session` from
    // src/kani-compiler/src/codegen_cprover_gotoc/context/goto_ctx.rs
    match session.target.options.min_global_align {
        Some(1) => (),
        Some(align) => {
            let err_msg = format!(
                "Kani requires the target architecture option `min_global_align` to be 1, but it is {}.",
                align
            );
            session.err(&err_msg);
        }
        _ => (),
    }

    if session.target.options.endian != Endian::Little {
        session.err("Kani requires the target architecture option `endian` to be `little`.");
    }

    if !session.overflow_checks() {
        session.err("Kani requires overflow checks in order to provide a sound analysis.");
    }

    if session.panic_strategy() != PanicStrategy::Abort {
        session.err(
            "Kani can only handle abort panic strategy (-C panic=abort). See for more details \
        https://github.com/model-checking/kani/issues/692",
        );
    }

    if need_metadata_module {
        session.err("Kani cannot generate metadata module.");
    }

    session.abort_if_errors();
}

fn crate_has_global_asm<'tcx>(codegen_units: &'tcx [CodegenUnit<'tcx>], tcx: TyCtxt<'tcx>) -> bool {
    for cgu in codegen_units {
        let items = cgu.items_in_deterministic_order(tcx);
        for (item, _) in items {
            if matches!(item, MonoItem::GlobalAsm(_)) {
                return true;
            }
        }
    }
    false
}

fn write_file<T>(base_filename: &PathBuf, extension: &str, source: &T, pretty: bool)
where
    T: serde::Serialize,
{
    let filename = base_filename.with_extension(extension);
    debug!("output to {:?}", filename);
    let out_file = ::std::fs::File::create(&filename).unwrap();
    let writer = BufWriter::new(out_file);
    if pretty {
        serde_json::to_writer_pretty(writer, &source).unwrap();
    } else {
        serde_json::to_writer(writer, &source).unwrap();
    }
}
