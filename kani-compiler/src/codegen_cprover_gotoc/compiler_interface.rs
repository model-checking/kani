// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use crate::args::ReachabilityType;
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::kani_middle::analysis;
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::check_reachable_items;
use crate::kani_middle::codegen_units::{CodegenUnit, CodegenUnits};
use crate::kani_middle::provide;
use crate::kani_middle::reachability::{collect_reachable_items, filter_crate_items};
use crate::kani_middle::transform::{BodyTransformation, GlobalPasses};
use crate::kani_queries::QueryDb;
use cbmc::RoundingMode;
use cbmc::goto_program::Location;
use cbmc::irep::goto_binary_serde::write_goto_binary_file;
use cbmc::{InternedString, MachineModel};
use kani_metadata::artifact::convert_type;
use kani_metadata::{ArtifactType, HarnessMetadata, KaniMetadata, UnsupportedFeature};
use kani_metadata::{AssignsContract, CompilerArtifactStub};
use rustc_abi::{Align, Endian};
use rustc_codegen_ssa::back::archive::{
    ArArchiveBuilder, ArchiveBuilder, ArchiveBuilderBuilder, DEFAULT_OBJECT_READER,
};
use rustc_codegen_ssa::back::link::link_binary;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_codegen_ssa::{CodegenResults, CrateInfo, TargetConfig};
use rustc_data_structures::fx::{FxHashMap, FxIndexMap};
use rustc_errors::DEFAULT_LOCALE_RESOURCE;
use rustc_hir::def_id::{DefId as InternalDefId, LOCAL_CRATE};
use rustc_metadata::EncodedMetadata;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::ty::TyCtxt;
use rustc_middle::util::Providers;
use rustc_session::Session;
use rustc_session::config::{CrateType, OutputFilenames, OutputType};
use rustc_session::output::out_filename;
use rustc_span::{Symbol, sym};
use rustc_target::spec::PanicStrategy;
use stable_mir::CrateDef;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::rustc_internal;
use stable_mir::ty::FnDef;
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};

pub type UnsupportedConstructs = FxHashMap<InternedString, Vec<Location>>;

#[derive(Clone)]
pub struct GotocCodegenBackend {
    /// The query is shared with `KaniCompiler` and it is initialized as part of `rustc`
    /// initialization, which may happen after this object is created.
    /// Since we don't have any guarantees on when the compiler creates the Backend object, neither
    /// in which thread it will be used, we prefer to explicitly synchronize any query access.
    queries: Arc<Mutex<QueryDb>>,
}

impl GotocCodegenBackend {
    pub fn new(queries: Arc<Mutex<QueryDb>>) -> Self {
        GotocCodegenBackend { queries }
    }

    /// Generate code that is reachable from the given starting points.
    ///
    /// Invariant: iff `check_contract.is_some()` then `return.2.is_some()`
    fn codegen_items<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        starting_items: &[MonoItem],
        symtab_goto: &Path,
        machine_model: &MachineModel,
        check_contract: Option<InternalDefId>,
        mut transformer: BodyTransformation,
    ) -> (GotocCtx<'tcx>, Vec<MonoItem>, Option<AssignsContract>) {
        // This runs reachability analysis before global passes are applied.
        //
        // Alternatively, we could run reachability only once after the global passes are applied
        // and resolve the necessary dependencies inside the passes on the fly. This, however, has a
        // disadvantage of not having a precomputed call graph for the global passes to use. The
        // call graph could be used, for example, in resolving function pointer or vtable calls for
        // global passes that need this.
        let (mut items, call_graph) = with_timer(
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
        let any_pass_modified = global_passes.run_global_passes(
            &mut transformer,
            tcx,
            starting_items,
            instances,
            call_graph,
        );

        // Re-collect reachable items after global transformations were applied. This is necessary
        // since global pass could add extra calls to instrumentation.
        if any_pass_modified {
            (items, _) = with_timer(
                || collect_reachable_items(tcx, &mut transformer, starting_items),
                "codegen reachability analysis (second pass)",
            );
        }

        // Follow rustc naming convention (cx is abbrev for context).
        // https://rustc-dev-guide.rust-lang.org/conventions.html#naming-conventions
        let mut gcx =
            GotocCtx::new(tcx, (*self.queries.lock().unwrap()).clone(), machine_model, transformer);
        check_reachable_items(gcx.tcx, &gcx.queries, &items);

        let contract_info = with_timer(
            || {
                // we first declare all items
                for item in &items {
                    match *item {
                        MonoItem::Fn(instance) => {
                            gcx.call_with_panic_debug_info(
                                |ctx| ctx.declare_function(instance),
                                format!("declare_function: {}", instance.name()),
                                instance.def,
                            );
                        }
                        MonoItem::Static(def) => {
                            gcx.call_with_panic_debug_info(
                                |ctx| ctx.declare_static(def),
                                format!("declare_static: {}", def.name()),
                                def,
                            );
                        }
                        MonoItem::GlobalAsm(_) => {} // Ignore this. We have already warned about it.
                    }
                }

                // then we move on to codegen
                for item in &items {
                    match *item {
                        MonoItem::Fn(instance) => {
                            gcx.call_with_panic_debug_info(
                                |ctx| ctx.codegen_function(instance),
                                format!(
                                    "codegen_function: {}\n{}",
                                    instance.name(),
                                    instance.mangled_name()
                                ),
                                instance.def,
                            );
                        }
                        MonoItem::Static(def) => {
                            gcx.call_with_panic_debug_info(
                                |ctx| ctx.codegen_static(def),
                                format!("codegen_static: {}", def.name()),
                                def,
                            );
                        }
                        MonoItem::GlobalAsm(_) => {} // We have already warned above
                    }
                }

                check_contract.map(|check_id| gcx.handle_check_contract(check_id, &items))
            },
            "codegen",
        );

        // Map from name to prettyName for all symbols
        let pretty_name_map: BTreeMap<InternedString, Option<InternedString>> =
            BTreeMap::from_iter(gcx.symbol_table.iter().map(|(k, s)| (*k, s.pretty_name)));

        // Map MIR types to GotoC types
        let type_map: BTreeMap<InternedString, InternedString> =
            BTreeMap::from_iter(gcx.type_map.iter().map(|(k, v)| (*k, v.to_string().into())));

        // Get the vtable function pointer restrictions if requested
        let vtable_restrictions = if gcx.vtable_ctx.emit_vtable_restrictions {
            Some(gcx.vtable_ctx.get_virtual_function_restrictions())
        } else {
            None
        };

        gcx.handle_quantifiers();

        // No output should be generated if user selected no_codegen.
        if !tcx.sess.opts.unstable_opts.no_codegen && tcx.sess.opts.output_types.should_codegen() {
            let pretty = self.queries.lock().unwrap().args().output_pretty_json;
            write_file(symtab_goto, ArtifactType::PrettyNameMap, &pretty_name_map, pretty);
            write_goto_binary_file(symtab_goto, &gcx.symbol_table);
            write_file(symtab_goto, ArtifactType::TypeMap, &type_map, pretty);
            // If they exist, write out vtable virtual call function pointer restrictions
            if let Some(restrictions) = vtable_restrictions {
                write_file(symtab_goto, ArtifactType::VTableRestriction, &restrictions, pretty);
            }
        }

        (gcx, items, contract_info)
    }

    /// Given a harness, return the DefId of its target if it's a contract harness.
    /// For manual harnesses, extract it from the #[proof_for_contract] attribute.
    /// For automatic harnesses, extract the target from the harness's GenericArgs.
    fn target_if_contract_harness(
        &self,
        tcx: TyCtxt,
        harness: &Instance,
        is_automatic_harness: bool,
    ) -> Option<FnDef> {
        if is_automatic_harness {
            let kind = harness.args().0[0].expect_ty().kind();
            let (fn_to_verify_def, _) = kind.fn_def().unwrap();
            let attrs = KaniAttributes::for_def_id(tcx, fn_to_verify_def.def_id());
            if attrs.has_contract() { Some(fn_to_verify_def) } else { None }
        } else {
            let harness_attrs = KaniAttributes::for_def_id(tcx, harness.def.def_id());
            harness_attrs.interpret_for_contract_attribute()
        }
    }
}

impl CodegenBackend for GotocCodegenBackend {
    fn provide(&self, providers: &mut Providers) {
        provide::provide(providers, &self.queries.lock().unwrap());
    }

    fn print_version(&self) {
        println!("Kani-goto version: {}", env!("CARGO_PKG_VERSION"));
    }

    fn locale_resource(&self) -> &'static str {
        // We don't currently support multiple languages.
        DEFAULT_LOCALE_RESOURCE
    }

    fn target_config(&self, sess: &Session) -> TargetConfig {
        // This code is adapted from the cranelift backend:
        // https://github.com/rust-lang/rust/blob/a124fb3cb7291d75872934f411d81fe298379ace/compiler/rustc_codegen_cranelift/src/lib.rs#L184
        let target_features = if sess.target.arch == "x86_64" && sess.target.os != "none" {
            // x86_64 mandates SSE2 support and rustc requires the x87 feature to be enabled
            vec![sym::sse, sym::sse2, Symbol::intern("x87")]
        } else if sess.target.arch == "aarch64" {
            match &*sess.target.os {
                "none" => vec![],
                // On macOS the aes, sha2 and sha3 features are enabled by default and ring
                // fails to compile on macOS when they are not present.
                "macos" => vec![sym::neon, sym::aes, sym::sha2, sym::sha3],
                // AArch64 mandates Neon support
                _ => vec![sym::neon],
            }
        } else {
            vec![]
        };
        // FIXME do `unstable_target_features` properly
        let unstable_target_features = target_features.clone();

        let has_reliable_f128 = true;
        let has_reliable_f16 = true;

        TargetConfig {
            target_features,
            unstable_target_features,
            has_reliable_f16,
            has_reliable_f16_math: has_reliable_f16,
            has_reliable_f128,
            has_reliable_f128_math: has_reliable_f128,
        }
    }

    fn codegen_crate(&self, tcx: TyCtxt) -> Box<dyn Any> {
        let ret_val = rustc_internal::run(tcx, || {
            super::utils::init();

            // Any changes to queries from this point on is just related to caching information
            // needed for generating code to the given crate.
            // The cached information must not outlive the stable-mir `run` scope.
            // See [QueryDb::kani_functions] for more information.
            let queries = self.queries.lock().unwrap().clone();

            check_target(tcx.sess);
            check_options(tcx.sess);
            if queries.args().reachability_analysis != ReachabilityType::None
                && queries.kani_functions().is_empty()
            {
                if stable_mir::find_crates("std").is_empty()
                    && stable_mir::find_crates("kani").is_empty()
                {
                    // Special error for when not importing kani and using #[no_std].
                    // See here for more info: https://github.com/model-checking/kani/issues/3906#issuecomment-2932687768.
                    tcx.sess.dcx().struct_err(
                        "Failed to detect Kani functions."
                    ).with_help(
                        "This project seems to be using #[no_std] but does not import Kani. \
                        Try adding `crate extern kani` to the crate root to explicitly import Kani."
                    )
                    .emit();
                } else {
                    tcx.sess.dcx().struct_err(
                        "Failed to detect Kani functions. Please check your installation is correct."
                    ).emit();
                }
            }

            // Codegen all items that need to be processed according to the selected reachability mode:
            //
            // - Harnesses: Generate one model per local harnesses (marked with `kani::proof` attribute).
            // - Tests: Generate one model per test harnesses.
            // - PubFns: Generate code for all reachable logic starting from the local public functions.
            // - None: Don't generate code. This is used to compile dependencies.
            let base_filepath = tcx.output_filenames(()).path(OutputType::Object);
            let base_filename = base_filepath.as_path();
            let reachability = queries.args().reachability_analysis;
            let mut results = GotoCodegenResults::new(tcx, reachability);
            match reachability {
                ReachabilityType::AllFns | ReachabilityType::Harnesses => {
                    let mut units = CodegenUnits::new(&queries, tcx);
                    let mut modifies_instances = vec![];
                    let mut loop_contracts_instances = vec![];
                    // Cross-crate collecting of all items that are reachable from the crate harnesses.
                    for unit in units.iter() {
                        // We reset the body cache for now because each codegen unit has different
                        // configurations that affect how we transform the instance body.
                        for harness in &unit.harnesses {
                            let transformer = BodyTransformation::new(&queries, tcx, unit);
                            let model_path = units.harness_model_path(*harness).unwrap();
                            let is_automatic_harness = units.is_automatic_harness(harness);
                            let contract_metadata =
                                self.target_if_contract_harness(tcx, harness, is_automatic_harness);
                            let (gcx, items, contract_info) = self.codegen_items(
                                tcx,
                                &[MonoItem::Fn(*harness)],
                                model_path,
                                &results.machine_model,
                                contract_metadata
                                    .map(|def| rustc_internal::internal(tcx, def.def_id())),
                                transformer,
                            );
                            if gcx.has_loop_contracts {
                                loop_contracts_instances.push(*harness);
                            }
                            results.extend(gcx, items, None);
                            if let Some(assigns_contract) = contract_info {
                                modifies_instances.push((*harness, assigns_contract));
                            }
                        }
                    }
                    units.store_modifies(&modifies_instances);
                    units.store_loop_contracts(&loop_contracts_instances);
                    units.write_metadata(&queries, tcx);
                }
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
                    let (gcx, items, contract_info) = self.codegen_items(
                        tcx,
                        &local_reachable,
                        &model_path,
                        &results.machine_model,
                        Default::default(),
                        transformer,
                    );
                    assert!(contract_info.is_none());
                    let _ = results.extend(gcx, items, None);
                }
            }

            if reachability != ReachabilityType::None {
                // Print compilation report.
                results.print_report(tcx);

                if reachability != ReachabilityType::Harnesses
                    && reachability != ReachabilityType::AllFns
                {
                    // In a workspace, cargo seems to be using the same file prefix to build a crate that is
                    // a package lib and also a dependency of another package.
                    // To avoid overriding the metadata for its verification, we skip this step when
                    // reachability is None, even because there is nothing to record.
                    write_file(
                        base_filename,
                        ArtifactType::Metadata,
                        &results.generate_metadata(),
                        queries.args().output_pretty_json,
                    );
                }
            }
            codegen_results(tcx, &results.machine_model)
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
    /// For other crate types, we stub the file requested by writing the
    /// path of the `kani-metadata.json` file so `kani-driver` can safely find the latest metadata.
    /// See <https://github.com/model-checking/kani/issues/2234> for more details.
    fn link(
        &self,
        sess: &Session,
        codegen_results: CodegenResults,
        rustc_metadata: EncodedMetadata,
        outputs: &OutputFilenames,
    ) {
        let requested_crate_types = &codegen_results.crate_info.crate_types.clone();
        let local_crate_name = codegen_results.crate_info.local_crate_name;
        // Create the rlib if one was requested.
        if requested_crate_types.contains(&CrateType::Rlib) {
            link_binary(sess, &ArArchiveBuilderBuilder, codegen_results, rustc_metadata, outputs);
        }

        // But override all the other outputs.
        // Note: Do this after `link_binary` call, since it may write to the object files
        // and override the json we are creating.
        for crate_type in requested_crate_types {
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

fn check_target(session: &Session) {
    // The requirement below is needed to build a valid CBMC machine model
    // in function `machine_model_from_session` from
    // src/kani-compiler/src/codegen_cprover_gotoc/context/goto_ctx.rs
    let is_x86_64_linux_target = session.target.llvm_target == "x86_64-unknown-linux-gnu";
    let is_arm64_linux_target = session.target.llvm_target == "aarch64-unknown-linux-gnu";
    // Comparison with `x86_64-apple-darwin` does not work well because the LLVM
    // target may become `x86_64-apple-macosx10.7.0` (or similar) and fail
    let is_x86_64_darwin_target = session.target.llvm_target.starts_with("x86_64-apple-");
    // looking for `arm64-apple-*`
    let is_arm64_darwin_target = session.target.llvm_target.starts_with("arm64-apple-");

    if !is_x86_64_linux_target
        && !is_arm64_linux_target
        && !is_x86_64_darwin_target
        && !is_arm64_darwin_target
    {
        let err_msg = format!(
            "Kani requires the target platform to be `x86_64-unknown-linux-gnu`, \
            `aarch64-unknown-linux-gnu`, `x86_64-apple-*` or `arm64-apple-*`, but \
            it is {}",
            &session.target.llvm_target
        );
        session.dcx().err(err_msg);
    }

    session.dcx().abort_if_errors();
}

fn check_options(session: &Session) {
    // The requirements for `min_global_align` and `endian` are needed to build
    // a valid CBMC machine model in function `machine_model_from_session` from
    // src/kani-compiler/src/codegen_cprover_gotoc/context/goto_ctx.rs
    match session.target.options.min_global_align {
        Some(Align::ONE) => (),
        Some(align) => {
            let err_msg = format!(
                "Kani requires the target architecture option `min_global_align` to be 1, but it is {}.",
                align.bytes()
            );
            session.dcx().err(err_msg);
        }
        _ => (),
    }

    if session.target.options.endian != Endian::Little {
        session.dcx().err("Kani requires the target architecture option `endian` to be `little`.");
    }

    if !session.overflow_checks() {
        session.dcx().err("Kani requires overflow checks in order to provide a sound analysis.");
    }

    if session.panic_strategy() != PanicStrategy::Abort {
        session.dcx().err(
            "Kani can only handle abort panic strategy (-C panic=abort). See for more details \
        https://github.com/model-checking/kani/issues/692",
        );
    }

    session.dcx().abort_if_errors();
}

/// Return a struct that contains information about the codegen results as expected by `rustc`.
fn codegen_results(tcx: TyCtxt, machine: &MachineModel) -> Box<dyn Any> {
    let work_products = FxIndexMap::<WorkProductId, WorkProduct>::default();
    Box::new((
        CodegenResults {
            modules: vec![],
            allocator_module: None,
            crate_info: CrateInfo::new(tcx, machine.architecture.clone()),
        },
        work_products,
    ))
}

pub fn write_file<T>(base_path: &Path, file_type: ArtifactType, source: &T, pretty: bool)
where
    T: serde::Serialize,
{
    let filename = convert_type(base_path, ArtifactType::SymTabGoto, file_type);
    debug!(?filename, "write_json");
    let out_file = File::create(&filename).unwrap();
    let writer = BufWriter::new(out_file);
    if pretty {
        serde_json::to_writer_pretty(writer, &source).unwrap();
    } else {
        serde_json::to_writer(writer, &source).unwrap();
    }
}

struct GotoCodegenResults {
    reachability: ReachabilityType,
    harnesses: Vec<HarnessMetadata>,
    unsupported_constructs: UnsupportedConstructs,
    concurrent_constructs: UnsupportedConstructs,
    items: Vec<MonoItem>,
    crate_name: InternedString,
    machine_model: MachineModel,
}

impl GotoCodegenResults {
    pub fn new(tcx: TyCtxt, reachability: ReachabilityType) -> Self {
        GotoCodegenResults {
            reachability,
            harnesses: vec![],
            unsupported_constructs: UnsupportedConstructs::default(),
            concurrent_constructs: UnsupportedConstructs::default(),
            items: vec![],
            crate_name: tcx.crate_name(LOCAL_CRATE).as_str().into(),
            machine_model: new_machine_model(tcx.sess),
        }
    }
    /// Method that generates `KaniMetadata` from the given compilation results.
    pub fn generate_metadata(&self) -> KaniMetadata {
        // Maps the goto-context "unsupported features" data into the KaniMetadata "unsupported features" format.
        // TODO: Do we really need different formats??
        let unsupported_features = self
            .unsupported_constructs
            .iter()
            .map(|(construct, location)| UnsupportedFeature {
                feature: construct.to_string(),
                locations: location
                    .iter()
                    .map(|l| {
                        // We likely (and should) have no instances of
                        // calling `codegen_unimplemented` without file/line.
                        // So while we map out of `Option` here, we expect them to always be `Some`
                        kani_metadata::Location {
                            filename: l.filename().unwrap_or_default(),
                            start_line: l.start_line().unwrap_or_default(),
                        }
                    })
                    .collect(),
            })
            .collect();
        let (proofs, tests) = if self.reachability == ReachabilityType::Harnesses {
            (self.harnesses.clone(), vec![])
        } else {
            (vec![], self.harnesses.clone())
        };
        KaniMetadata {
            crate_name: self.crate_name.to_string(),
            proof_harnesses: proofs,
            unsupported_features,
            test_harnesses: tests,
            // We don't collect the contracts metadata because the FunctionWithContractPass
            // removes any contracts logic for ReachabilityType::PubFns,
            // which is the only ReachabilityType under which the compiler calls this function.
            contracted_functions: vec![],
            autoharness_md: None,
        }
    }

    fn extend(
        &mut self,
        gcx: GotocCtx,
        items: Vec<MonoItem>,
        metadata: Option<HarnessMetadata>,
    ) -> BodyTransformation {
        let mut items = items;
        self.harnesses.extend(metadata);
        self.concurrent_constructs.extend(gcx.concurrent_constructs);
        self.unsupported_constructs.extend(gcx.unsupported_constructs);
        self.items.append(&mut items);
        gcx.transformer
    }

    /// Prints a report at the end of the compilation.
    fn print_report(&self, tcx: TyCtxt) {
        // Print all unsupported constructs.
        if !self.unsupported_constructs.is_empty() {
            // Sort alphabetically.
            let unsupported: BTreeMap<String, &Vec<Location>> = self
                .unsupported_constructs
                .iter()
                .map(|(key, val)| (key.map(|s| String::from(s)), val))
                .collect();
            let mut msg = String::from("Found the following unsupported constructs:\n");
            unsupported.iter().for_each(|(construct, locations)| {
                writeln!(&mut msg, "    - {construct} ({})", locations.len()).unwrap();
            });
            msg += "\nVerification will fail if one or more of these constructs is reachable.";
            msg += "\nSee https://model-checking.github.io/kani/rust-feature-support.html for more \
            details.";
            tcx.dcx().warn(msg);
        }

        if !self.concurrent_constructs.is_empty() {
            let mut msg = String::from(
                "Kani currently does not support concurrency. The following constructs will be treated \
                as sequential operations:\n",
            );
            for (construct, locations) in self.concurrent_constructs.iter() {
                writeln!(&mut msg, "    - {construct} ({})", locations.len()).unwrap();
            }
            tcx.dcx().warn(msg);
        }

        // Print some compilation stats.
        if tracing::enabled!(tracing::Level::INFO) {
            analysis::print_stats(&self.items);
        }
    }
}

/// Builds a machine model which is required by CBMC
fn new_machine_model(sess: &Session) -> MachineModel {
    // The model assumes a `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`
    // or `aarch64-apple-darwin` platform. We check the target platform in function
    // `check_target` from src/kani-compiler/src/codegen_cprover_gotoc/compiler_interface.rs
    // and error if it is not any of the ones we expect.
    let architecture = &sess.target.arch;
    let os = &sess.target.os;
    let pointer_width = sess.target.pointer_width.into();

    // The model assumes the following values for session options:
    //   * `min_global_align`: 1
    //   * `endian`: `Endian::Little`
    //
    // We check these options in function `check_options` from
    // src/kani-compiler/src/codegen_cprover_gotoc/compiler_interface.rs
    // and error if their values are not the ones we expect.
    let alignment = sess.target.options.min_global_align.map_or(1, |align| align.bytes());
    let is_big_endian = match sess.target.options.endian {
        Endian::Little => false,
        Endian::Big => true,
    };

    // The values below cannot be obtained from the session so they are
    // hardcoded using standard ones for the supported platforms
    // see /tools/sizeofs/main.cpp.
    // For reference, the definition in CBMC:
    //https://github.com/diffblue/cbmc/blob/develop/src/util/config.cpp
    match architecture.as_ref() {
        "x86_64" => {
            let bool_width = 8;
            let char_is_unsigned = false;
            let char_width = 8;
            let double_width = 64;
            let float_width = 32;
            let int_width = 32;
            let long_double_width = 128;
            let long_int_width = 64;
            let long_long_int_width = 64;
            let short_int_width = 16;
            let single_width = 32;
            let wchar_t_is_unsigned = false;
            let wchar_t_width = 32;

            MachineModel {
                architecture: architecture.to_string(),
                alignment,
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
                memory_operand_size: int_width / 8,
                null_is_zero: true,
                pointer_width,
                rounding_mode: RoundingMode::ToNearest,
                short_int_width,
                single_width,
                wchar_t_is_unsigned,
                wchar_t_width,
                word_size: int_width,
            }
        }
        "aarch64" => {
            let bool_width = 8;
            let char_is_unsigned = true;
            let char_width = 8;
            let double_width = 64;
            let float_width = 32;
            let int_width = 32;
            let long_double_width = match os.as_ref() {
                "linux" => 128,
                _ => 64,
            };
            let long_int_width = 64;
            let long_long_int_width = 64;
            let short_int_width = 16;
            let single_width = 32;
            // https://developer.arm.com/documentation/dui0491/i/Compiler-Command-line-Options/--signed-chars----unsigned-chars
            // https://www.arm.linux.org.uk/docs/faqs/signedchar.php
            // https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms
            let wchar_t_is_unsigned = matches!(os.as_ref(), "linux");
            let wchar_t_width = 32;

            MachineModel {
                // CBMC calls it arm64, not aarch64
                architecture: "arm64".to_string(),
                alignment,
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
                memory_operand_size: int_width / 8,
                null_is_zero: true,
                pointer_width,
                rounding_mode: RoundingMode::ToNearest,
                short_int_width,
                single_width,
                wchar_t_is_unsigned,
                wchar_t_width,
                word_size: int_width,
            }
        }
        _ => {
            panic!("Unsupported architecture: {architecture}");
        }
    }
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
