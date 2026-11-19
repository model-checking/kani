// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use crate::args::ReachabilityType;
use crate::codegen_aeneas_llbc::mir_to_ullbc::{
    Context, prepare_translated_crate, record_item_names,
};
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::check_reachable_items;
use crate::kani_middle::codegen_units::{CodegenUnit, CodegenUnits};
use crate::kani_middle::provide;
use crate::kani_middle::reachability::{collect_reachable_items, filter_crate_items};
use crate::kani_middle::transform::{BodyTransformation, GlobalPasses};
use crate::kani_queries::QUERY_DB;
use charon_lib::ast::{ItemId, TranslatedCrate};
use charon_lib::errors::ErrorCtx;
use charon_lib::options::{CliOpts, Preset, SerializationFormat, TranslateOptions};
use charon_lib::transform::{TransformCtx, run_transformation_passes};
use kani_metadata::ArtifactType;
use kani_metadata::{AssignsContract, CompilerArtifactStub};
use rustc_codegen_ssa::back::archive::{
    ArArchiveBuilder, ArchiveBuilder, ArchiveBuilderBuilder, DEFAULT_OBJECT_READER,
};
use rustc_codegen_ssa::back::link::link_binary;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_codegen_ssa::{CompiledModules, CrateInfo};
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::unord::UnordMap;
use rustc_errors::ErrorGuaranteed;
use rustc_hir::def_id::{DefId as InternalDefId, LOCAL_CRATE};
use rustc_metadata::EncodedMetadata;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::ty::TyCtxt;
use rustc_middle::util::Providers;
use rustc_public::mir::mono::{Instance, MonoItem};
use rustc_public::rustc_internal;
use rustc_public::ty::FnDef;
use rustc_public::{CrateDef, DefId};
use rustc_session::config::{OutputFilenames, OutputType};
use rustc_session::output::out_filename;
use rustc_session::{IncrCompSession, Session};
use rustc_structures::CrateType;
use std::any::Any;
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};

#[derive(Clone)]
pub struct LlbcCodegenBackend {}

impl LlbcCodegenBackend {
    pub fn new() -> Self {
        LlbcCodegenBackend {}
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
        QUERY_DB.with(|db| {
            let mut global_passes = GlobalPasses::new(&db.borrow(), tcx);
            global_passes.run_global_passes(
                &mut transformer,
                tcx,
                starting_items,
                instances,
                call_graph,
            );
        });

        let queries = QUERY_DB.with(|db| db.borrow().clone());
        check_reachable_items(tcx, &queries, &items);

        // Follow rustc naming convention (cx is abbrev for context).
        // https://rustc-dev-guide.rust-lang.org/conventions.html#naming-conventions

        // Create a Charon transformation context that will be populated with translation results
        let mut ccx = create_charon_transformation_context(tcx);
        let mut id_map: FxHashMap<DefId, ItemId> = FxHashMap::default();

        // Translate all the items
        for item in &items {
            debug!("Translating: {item:?}");
            match item {
                MonoItem::Fn(instance) => {
                    let mut errors_borrow = ccx.errors.borrow_mut();
                    let mut fcx = Context::new(
                        tcx,
                        *instance,
                        &mut ccx.translated,
                        &mut id_map,
                        &mut errors_borrow,
                    );
                    let _ = fcx.translate();
                }
                MonoItem::Static(_def) => todo!(),
                MonoItem::GlobalAsm(_) => {} // We have already warned above
            }
        }

        record_item_names(&mut ccx.translated);

        // Everything after translation is Charon's own pipeline, run exactly as `charon` runs it,
        // so that the LLBC we emit is what Aeneas expects and a Charon bump does not require
        // re-deriving its pass list here.
        run_transformation_passes(&charon_cli_options(queries.args().print_llbc), &mut ccx);

        // Charon has already printed each error, including those of its type check. Stop here
        // rather than emit LLBC that Charon considers ill-formed.
        // TODO: display an error report about the external dependencies, if necessary
        let error_count = ccx.errors.borrow().error_count;
        if error_count > 0 {
            tcx.dcx()
                .fatal(format!("Charon reported {error_count} error(s) while translating to LLBC"));
        }

        let crate_data: charon_lib::export::CrateData = charon_lib::export::CrateData::new(ccx);

        // No output should be generated if user selected no_codegen.
        if !tcx.sess.opts.unstable_opts.no_codegen && tcx.sess.opts.output_types.should_codegen() {
            // # Final step: generate the files.
            // `crate_data` is set by our callbacks when there is no fatal error.
            let mut pb = llbc_file.to_path_buf();
            pb.set_extension("llbc");
            println!("Writing LLBC file to {}", pb.display());
            if let Err(()) = crate_data.serialize_to_file(&pb, SerializationFormat::Json) {
                tcx.sess.dcx().err("Failed to write LLBC file");
            }
        }

        (items, None)
    }
}

impl CodegenBackend for LlbcCodegenBackend {
    fn provide(&self, providers: &mut Providers) {
        QUERY_DB.with(|db| provide::provide(providers, &db.borrow()));
    }

    fn print_version(&self) {
        println!("Kani-llbc version: {}", env!("CARGO_PKG_VERSION"));
    }

    fn name(&self) -> &'static str {
        "kani-llbc"
    }

    fn target_cpu(&self, sess: &Session) -> String {
        match sess.opts.cg.target_cpu {
            Some(ref name) => name,
            None => sess.target.cpu.as_ref(),
        }
        .to_owned()
    }

    fn codegen_crate<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Box<dyn Any> {
        let ret_val = rustc_internal::run(tcx, || {
            // Queries shouldn't change today once codegen starts.
            let queries = QUERY_DB.with(|db| db.borrow().clone());

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
                                contract_metadata
                                    .map(|def| rustc_internal::internal(tcx, def.def_id())),
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
                ReachabilityType::AllFns => todo!(),
                ReachabilityType::None => {}
                ReachabilityType::PubFns => {
                    let unit = CodegenUnit::default();
                    let transformer = BodyTransformation::new(&queries, tcx, &unit);
                    let main_instance = rustc_public::entry_fn()
                        .map(|main_fn| Instance::try_from(main_fn).unwrap());
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
            codegen_results()
        });
        ret_val.unwrap()
    }

    fn join_codegen(
        &self,
        ongoing_codegen: Box<dyn Any>,
        _sess: &Session,
        _incr_comp_session: Option<&IncrCompSession>,
        _filenames: &OutputFilenames,
        _crate_info: &CrateInfo,
    ) -> (CompiledModules, UnordMap<WorkProductId, WorkProduct>) {
        match ongoing_codegen.downcast::<(CompiledModules, UnordMap<WorkProductId, WorkProduct>)>()
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
        compiled_modules: CompiledModules,
        crate_info: CrateInfo,
        rustc_metadata: EncodedMetadata,
        outputs: &OutputFilenames,
    ) {
        let requested_crate_types = crate_info.crate_types.clone();
        let local_crate_name = crate_info.local_crate_name;
        link_binary(
            sess,
            &ArArchiveBuilderBuilder,
            compiled_modules,
            crate_info,
            rustc_metadata,
            outputs,
            self.name(),
        );
        for crate_type in &requested_crate_types {
            let out_fname = out_filename(sess, *crate_type, outputs, local_crate_name);
            let out_path = out_fname.as_path();
            debug!(?crate_type, ?out_path, "link");
            if *crate_type != CrateType::Rlib {
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
    }
}

struct ArArchiveBuilderBuilder;
impl ArchiveBuilderBuilder for ArArchiveBuilderBuilder {
    fn new_archive_builder<'a>(&self, sess: &'a Session) -> Box<dyn ArchiveBuilder + 'a> {
        Box::new(ArArchiveBuilder::new(sess, &DEFAULT_OBJECT_READER))
    }
}

fn contract_metadata_for_harness(
    tcx: TyCtxt,
    def_id: DefId,
) -> Result<Option<FnDef>, ErrorGuaranteed> {
    let attrs = KaniAttributes::for_def_id(tcx, def_id);
    Ok(attrs.interpret_for_contract_attribute())
}

/// Return a struct that contains information about the codegen results as expected by `rustc`.
///
/// Kani produces no object files, so the module lists are empty. `rustc` now builds the `CrateInfo`
/// itself and passes it to `codegen_crate` and `link`, so there is nothing crate-specific to report
/// here.
fn codegen_results() -> Box<dyn Any> {
    let work_products = UnordMap::<WorkProductId, WorkProduct>::default();
    Box::new((CompiledModules { modules: vec![], allocator_module: None }, work_products))
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

/// The Charon options Kani runs with: Charon's `aeneas` preset, which is the configuration Aeneas
/// consumes -- including which items are opaque and which traits are hidden (`Allocator` and the
/// marker traits), so Kani does not keep a copy of that policy. `print_llbc` makes the final pass
/// pipeline print the LLBC, as `charon --print-llbc` does; the expected tests rely on it.
fn charon_cli_options(print_llbc: bool) -> CliOpts {
    let mut options = CliOpts { preset: Some(Preset::Aeneas), print_llbc, ..CliOpts::default() };
    options.apply_preset();
    options
}

fn create_charon_transformation_context(tcx: TyCtxt) -> TransformCtx {
    let crate_name = tcx.crate_name(LOCAL_CRATE).as_str().into();
    let mut translated = TranslatedCrate { crate_name, ..TranslatedCrate::default() };
    prepare_translated_crate(tcx, &mut translated);
    let mut errors = ErrorCtx::new();
    let options = TranslateOptions::new(&mut errors, &charon_cli_options(false));
    TransformCtx { options, translated, errors: std::cell::RefCell::new(errors) }
}
