// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use crate::args::ReachabilityType;
use crate::codegen_aeneas_llbc::mir_to_ullbc::Context;
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::check_reachable_items;
use crate::kani_middle::codegen_units::{CodegenUnit, CodegenUnits};
use crate::kani_middle::provide;
use crate::kani_middle::reachability::{collect_reachable_items, filter_crate_items};
use crate::kani_middle::transform::{BodyTransformation, GlobalPasses};
use crate::kani_queries::QueryDb;
use charon_lib::ast::TranslatedCrate;
use charon_lib::errors::ErrorCtx;
use charon_lib::transform::ctx::TransformOptions;
use charon_lib::transform::TransformCtx;
use kani_metadata::ArtifactType;
use kani_metadata::{AssignsContract, CompilerArtifactStub};
use rustc_codegen_ssa::back::archive::{ArArchiveBuilder, ArchiveBuilder, DEFAULT_OBJECT_READER};
use rustc_codegen_ssa::back::metadata::create_wrapper_file;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_codegen_ssa::{CodegenResults, CrateInfo};
use rustc_data_structures::fx::FxIndexMap;
use rustc_data_structures::temp_dir::MaybeTempDir;
use rustc_errors::{ErrorGuaranteed, DEFAULT_LOCALE_RESOURCE};
use rustc_hir::def_id::{DefId as InternalDefId, LOCAL_CRATE};
use rustc_metadata::creader::MetadataLoaderDyn;
use rustc_metadata::fs::{emit_wrapper_file, METADATA_FILENAME};
use rustc_metadata::EncodedMetadata;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::ty::TyCtxt;
use rustc_middle::util::Providers;
use rustc_session::config::{CrateType, OutputFilenames, OutputType};
use rustc_session::output::out_filename;
use rustc_session::Session;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::{CrateDef, DefId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tempfile::Builder as TempFileBuilder;
use tracing::{debug, info, trace};

#[derive(Clone)]
pub struct LlbcCodegenBackend {
    /// The query is shared with `KaniCompiler` and it is initialized as part of `rustc`
    /// initialization, which may happen after this object is created.
    /// Since we don't have any guarantees on when the compiler creates the Backend object, neither
    /// in which thread it will be used, we prefer to explicitly synchronize any query access.
    queries: Arc<Mutex<QueryDb>>,
}

impl LlbcCodegenBackend {
    pub fn new(queries: Arc<Mutex<QueryDb>>) -> Self {
        LlbcCodegenBackend { queries }
    }

    /// Generate code that is reachable from the given starting points.
    ///
    /// Invariant: iff `check_contract.is_some()` then `return.2.is_some()`
    fn codegen_items(
        &self,
        tcx: TyCtxt,
        starting_items: &[MonoItem],
        llbc_file: &Path,
        _check_contract: Option<InternalDefId>,
        mut transformer: BodyTransformation,
    ) -> (Vec<MonoItem>, Option<AssignsContract>) {
        let (items, call_graph) = with_timer(
            || collect_reachable_items(tcx, &mut transformer, starting_items),
            "codegen reachability analysis",
        );

        // Retrieve all instances from the currently codegened items.
        let instances = items
            .iter()
            .filter_map(|item| match item {
                MonoItem::Fn(instance) => Some(*instance),
                MonoItem::Static(static_def) => {
                    let instance: Instance = (*static_def).into();
                    instance.has_body().then_some(instance)
                }
                MonoItem::GlobalAsm(_) => None,
            })
            .collect();

        // Apply all transformation passes, including global passes.
        let mut global_passes = GlobalPasses::new(&self.queries.lock().unwrap(), tcx);
        global_passes.run_global_passes(
            &mut transformer,
            tcx,
            starting_items,
            instances,
            call_graph,
        );

        let queries = self.queries.lock().unwrap().clone();
        check_reachable_items(tcx, &queries, &items);

        // Follow rustc naming convention (cx is abbrev for context).
        // https://rustc-dev-guide.rust-lang.org/conventions.html#naming-conventions

        // Create a Charon transformation context that will be populated with translation results
        let mut ccx = create_charon_transformation_context(tcx);

        // Translate all the items
        for item in &items {
            match item {
                MonoItem::Fn(instance) => {
                    let mut fcx =
                        Context::new(tcx, *instance, &mut ccx.translated, &mut ccx.errors);
                    let _ = fcx.translate();
                }
                MonoItem::Static(_def) => todo!(),
                MonoItem::GlobalAsm(_) => {} // We have already warned above
            }
        }

        trace!("# ULLBC after translation from MIR:\n\n{}\n", ccx);

        // # Reorder the graph of dependencies and compute the strictly
        // connex components to:
        // - compute the order in which to extract the definitions
        // - find the recursive definitions
        // - group the mutually recursive definitions
        let reordered_decls = charon_lib::reorder_decls::compute_reordered_decls(&ccx);
        ccx.translated.ordered_decls = Some(reordered_decls);

        //
        // =================
        // **Micro-passes**:
        // =================
        // At this point, the bulk of the translation is done. From now onwards,
        // we simply apply some micro-passes to make the code cleaner, before
        // serializing the result.

        // Run the micro-passes that clean up bodies.
        for pass in charon_lib::transform::ULLBC_PASSES.iter() {
            pass.transform_ctx(&mut ccx)
        }

        // # Go from ULLBC to LLBC (Low-Level Borrow Calculus) by reconstructing
        // the control flow.
        charon_lib::ullbc_to_llbc::translate_functions(&mut ccx);

        trace!("# LLBC resulting from control-flow reconstruction:\n\n{}\n", ccx);

        // Run the micro-passes that clean up bodies.
        for pass in charon_lib::transform::LLBC_PASSES.iter() {
            pass.transform_ctx(&mut ccx)
        }

        // Print the LLBC if requested. This is useful for expected tests.
        if queries.args().print_llbc {
            println!("# Final LLBC before serialization:\n\n{}\n", ccx);
        } else {
            debug!("# Final LLBC before serialization:\n\n{}\n", ccx);
        }

        // Display an error report about the external dependencies, if necessary
        ccx.errors.report_external_deps_errors();

        let crate_data: charon_lib::export::CrateData = charon_lib::export::CrateData::new(&ccx);

        // No output should be generated if user selected no_codegen.
        if !tcx.sess.opts.unstable_opts.no_codegen && tcx.sess.opts.output_types.should_codegen() {
            // # Final step: generate the files.
            // `crate_data` is set by our callbacks when there is no fatal error.
            let mut pb = llbc_file.to_path_buf();
            pb.set_extension("llbc");
            println!("Writing LLBC file to {}", pb.display());
            if let Err(()) = crate_data.serialize_to_file(&pb) {
                tcx.sess.dcx().err("Failed to write LLBC file");
            }
        }

        (items, None)
    }
}

impl CodegenBackend for LlbcCodegenBackend {
    fn metadata_loader(&self) -> Box<MetadataLoaderDyn> {
        Box::new(rustc_codegen_ssa::back::metadata::DefaultMetadataLoader)
    }

    fn provide(&self, providers: &mut Providers) {
        provide::provide(providers, &self.queries.lock().unwrap());
    }

    fn print_version(&self) {
        println!("Kani-llbc version: {}", env!("CARGO_PKG_VERSION"));
    }

    fn locale_resource(&self) -> &'static str {
        // We don't currently support multiple languages.
        DEFAULT_LOCALE_RESOURCE
    }

    fn codegen_crate(
        &self,
        tcx: TyCtxt,
        rustc_metadata: EncodedMetadata,
        _need_metadata_module: bool,
    ) -> Box<dyn Any> {
        let ret_val = rustc_internal::run(tcx, || {
            // Queries shouldn't change today once codegen starts.
            let queries = self.queries.lock().unwrap().clone();

            // Codegen all items that need to be processed according to the selected reachability mode:
            //
            // - Harnesses: Generate one model per local harnesses (marked with `kani::proof` attribute).
            // - Tests: Generate one model per test harnesses.
            // - PubFns: Generate code for all reachable logic starting from the local public functions.
            // - None: Don't generate code. This is used to compile dependencies.
            let base_filepath = tcx.output_filenames(()).path(OutputType::Object);
            let base_filename = base_filepath.as_path();
            let reachability = queries.args().reachability_analysis;
            match reachability {
                ReachabilityType::Harnesses => {
                    let mut units = CodegenUnits::new(&queries, tcx);
                    let modifies_instances = vec![];
                    // Cross-crate collecting of all items that are reachable from the crate harnesses.
                    for unit in units.iter() {
                        // We reset the body cache for now because each codegen unit has different
                        // configurations that affect how we transform the instance body.
                        let mut transformer = BodyTransformation::new(&queries, tcx, &unit);
                        for harness in &unit.harnesses {
                            let model_path = units.harness_model_path(*harness).unwrap();
                            let contract_metadata =
                                contract_metadata_for_harness(tcx, harness.def.def_id()).unwrap();
                            let (_items, contract_info) = self.codegen_items(
                                tcx,
                                &[MonoItem::Fn(*harness)],
                                model_path,
                                contract_metadata,
                                transformer,
                            );
                            transformer = BodyTransformation::new(&queries, tcx, &unit);
                            if let Some(_assigns_contract) = contract_info {
                                //self.queries.lock().unwrap().register_assigns_contract(
                                //    canonical_mangled_name(harness).intern(),
                                //    assigns_contract,
                                //);
                            }
                        }
                    }
                    units.store_modifies(&modifies_instances);
                    units.write_metadata(&queries, tcx);
                }
                ReachabilityType::Tests => todo!(),
                ReachabilityType::None => {}
                ReachabilityType::PubFns => {
                    let unit = CodegenUnit::default();
                    let transformer = BodyTransformation::new(&queries, tcx, &unit);
                    let main_instance =
                        stable_mir::entry_fn().map(|main_fn| Instance::try_from(main_fn).unwrap());
                    let local_reachable = filter_crate_items(tcx, |_, instance| {
                        let def_id = rustc_internal::internal(tcx, instance.def.def_id());
                        Some(instance) == main_instance || tcx.is_reachable_non_generic(def_id)
                    })
                    .into_iter()
                    .map(MonoItem::Fn)
                    .collect::<Vec<_>>();
                    let model_path = base_filename.with_extension(ArtifactType::SymTabGoto);
                    let (_items, contract_info) = self.codegen_items(
                        tcx,
                        &local_reachable,
                        &model_path,
                        Default::default(),
                        transformer,
                    );
                    assert!(contract_info.is_none());
                }
            }

            if reachability != ReachabilityType::None && reachability != ReachabilityType::Harnesses
            {
                // In a workspace, cargo seems to be using the same file prefix to build a crate that is
                // a package lib and also a dependency of another package.
                // To avoid overriding the metadata for its verification, we skip this step when
                // reachability is None, even because there is nothing to record.
            }
            codegen_results(tcx, rustc_metadata)
        });
        ret_val.unwrap()
    }

    fn join_codegen(
        &self,
        ongoing_codegen: Box<dyn Any>,
        _sess: &Session,
        _filenames: &OutputFilenames,
    ) -> (CodegenResults, FxIndexMap<WorkProductId, WorkProduct>) {
        match ongoing_codegen.downcast::<(CodegenResults, FxIndexMap<WorkProductId, WorkProduct>)>()
        {
            Ok(val) => *val,
            Err(val) => panic!("unexpected error: {:?}", (*val).type_id()),
        }
    }

    /// Emit output files during the link stage if it was requested.
    ///
    /// We need to emit `rlib` files normally if requested. Cargo expects these in some
    /// circumstances and sends them to subsequent builds with `-L`.
    ///
    /// We CAN NOT invoke the native linker, because that will fail. We don't have real objects.
    /// What determines whether the native linker is invoked or not is the set of `crate_types`.
    /// Types such as `bin`, `cdylib`, `dylib` will trigger the native linker.
    ///
    /// Thus, we manually build the rlib file including only the `rmeta` file.
    ///
    /// For cases where no metadata file was requested, we stub the file requested by writing the
    /// path of the `kani-metadata.json` file so `kani-driver` can safely find the latest metadata.
    /// See <https://github.com/model-checking/kani/issues/2234> for more details.
    fn link(
        &self,
        sess: &Session,
        codegen_results: CodegenResults,
        outputs: &OutputFilenames,
    ) -> Result<(), ErrorGuaranteed> {
        let requested_crate_types = &codegen_results.crate_info.crate_types;
        for crate_type in requested_crate_types {
            let out_fname = out_filename(
                sess,
                *crate_type,
                outputs,
                codegen_results.crate_info.local_crate_name,
            );
            let out_path = out_fname.as_path();
            debug!(?crate_type, ?out_path, "link");
            if *crate_type == CrateType::Rlib {
                // Emit the `rlib` that contains just one file: `<crate>.rmeta`
                let mut builder = Box::new(ArArchiveBuilder::new(sess, &DEFAULT_OBJECT_READER));
                let tmp_dir = TempFileBuilder::new().prefix("kani").tempdir().unwrap();
                let path = MaybeTempDir::new(tmp_dir, sess.opts.cg.save_temps);
                let (metadata, _metadata_position) = create_wrapper_file(
                    sess,
                    ".rmeta".to_string(),
                    codegen_results.metadata.raw_data(),
                );
                let metadata = emit_wrapper_file(sess, &metadata, &path, METADATA_FILENAME);
                builder.add_file(&metadata);
                builder.build(&out_path);
            } else {
                // Write the location of the kani metadata file in the requested compiler output file.
                let base_filepath = outputs.path(OutputType::Object);
                let base_filename = base_filepath.as_path();
                let content_stub = CompilerArtifactStub {
                    metadata_path: base_filename.with_extension(ArtifactType::Metadata),
                };
                let out_file = File::create(out_path).unwrap();
                serde_json::to_writer(out_file, &content_stub).unwrap();
            }
        }
        Ok(())
    }
}

fn contract_metadata_for_harness(
    tcx: TyCtxt,
    def_id: DefId,
) -> Result<Option<InternalDefId>, ErrorGuaranteed> {
    let attrs = KaniAttributes::for_def_id(tcx, def_id);
    Ok(attrs.interpret_for_contract_attribute().map(|(_, id, _)| id))
}

/// Return a struct that contains information about the codegen results as expected by `rustc`.
fn codegen_results(tcx: TyCtxt, rustc_metadata: EncodedMetadata) -> Box<dyn Any> {
    let work_products = FxIndexMap::<WorkProductId, WorkProduct>::default();
    Box::new((
        CodegenResults {
            modules: vec![],
            allocator_module: None,
            metadata_module: None,
            metadata: rustc_metadata,
            crate_info: CrateInfo::new(tcx, tcx.sess.target.arch.clone().to_string()),
        },
        work_products,
    ))
}

/// Execute the provided function and measure the clock time it took for its execution.
/// Log the time with the given description.
pub fn with_timer<T, F>(func: F, description: &str) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let ret = func();
    let elapsed = start.elapsed();
    info!("Finished {description} in {}s", elapsed.as_secs_f32());
    ret
}

fn create_charon_transformation_context(tcx: TyCtxt) -> TransformCtx {
    let options = TransformOptions {
        no_code_duplication: false,
        hide_marker_traits: false,
        item_opacities: Vec::new(),
    };
    let crate_name = tcx.crate_name(LOCAL_CRATE).as_str().into();
    let translated = TranslatedCrate { crate_name, ..TranslatedCrate::default() };
    let errors = ErrorCtx {
        continue_on_failure: true,
        errors_as_warnings: false,
        dcx: tcx.dcx(),
        decls_with_errors: HashSet::new(),
        ignored_failed_decls: HashSet::new(),
        dep_sources: HashMap::new(),
        def_id: None,
        error_count: 0,
    };
    TransformCtx { options, translated, errors }
}
