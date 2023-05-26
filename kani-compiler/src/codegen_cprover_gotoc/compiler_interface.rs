// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use crate::codegen_cprover_gotoc::GotocCtx;
use crate::kani_middle::analysis;
use crate::kani_middle::attributes::is_proof_harness;
use crate::kani_middle::attributes::is_test_harness_description;
use crate::kani_middle::check_crate_items;
use crate::kani_middle::check_reachable_items;
use crate::kani_middle::metadata::{gen_proof_metadata, gen_test_metadata};
use crate::kani_middle::provide;
use crate::kani_middle::reachability::{
    collect_reachable_items, filter_const_crate_items, filter_crate_items,
};
use cbmc::goto_program::Location;
use cbmc::irep::goto_binary_serde::write_goto_binary_file;
use cbmc::RoundingMode;
use cbmc::{InternedString, MachineModel};
use kani_metadata::artifact::convert_type;
use kani_metadata::CompilerArtifactStub;
use kani_metadata::UnsupportedFeature;
use kani_metadata::{ArtifactType, HarnessMetadata, KaniMetadata};
use kani_queries::{QueryDb, ReachabilityType, UserInput};
use rustc_codegen_ssa::back::archive::{
    get_native_object_symbols, ArArchiveBuilder, ArchiveBuilder,
};
use rustc_codegen_ssa::back::metadata::create_wrapper_file;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_codegen_ssa::{CodegenResults, CrateInfo};
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::temp_dir::MaybeTempDir;
use rustc_errors::{ErrorGuaranteed, DEFAULT_LOCALE_RESOURCE};
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_metadata::fs::{emit_wrapper_file, METADATA_FILENAME};
use rustc_metadata::EncodedMetadata;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::mir::mono::MonoItem;
use rustc_middle::mir::write_mir_pretty;
use rustc_middle::ty::query::Providers;
use rustc_middle::ty::{self, InstanceDef, TyCtxt};
use rustc_session::config::{CrateType, OutputFilenames, OutputType};
use rustc_session::cstore::MetadataLoaderDyn;
use rustc_session::output::out_filename;
use rustc_session::Session;
use rustc_span::def_id::DefId;
use rustc_target::abi::Endian;
use rustc_target::spec::PanicStrategy;
use std::any::Any;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write as IoWrite;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tempfile::Builder as TempFileBuilder;
use tracing::{debug, error, info};

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
    fn codegen_items<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        starting_items: &[MonoItem<'tcx>],
        symtab_goto: &Path,
        machine_model: &MachineModel,
    ) -> (GotocCtx<'tcx>, Vec<MonoItem<'tcx>>) {
        let items = with_timer(
            || collect_reachable_items(tcx, starting_items),
            "codegen reachability analysis",
        );
        dump_mir_items(tcx, &items);

        // Follow rustc naming convention (cx is abbrev for context).
        // https://rustc-dev-guide.rust-lang.org/conventions.html#naming-conventions
        let mut gcx = GotocCtx::new(tcx, (*self.queries.lock().unwrap()).clone(), machine_model);
        check_reachable_items(gcx.tcx, &gcx.queries, &items);

        with_timer(
            || {
                // we first declare all items
                for item in &items {
                    match *item {
                        MonoItem::Fn(instance) => {
                            gcx.call_with_panic_debug_info(
                                |ctx| ctx.declare_function(instance),
                                format!(
                                    "declare_function: {}",
                                    gcx.readable_instance_name(instance)
                                ),
                                instance.def_id(),
                            );
                        }
                        MonoItem::Static(def_id) => {
                            gcx.call_with_panic_debug_info(
                                |ctx| ctx.declare_static(def_id, *item),
                                format!("declare_static: {def_id:?}"),
                                def_id,
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
                                    gcx.readable_instance_name(instance),
                                    gcx.symbol_name(instance)
                                ),
                                instance.def_id(),
                            );
                        }
                        MonoItem::Static(def_id) => {
                            gcx.call_with_panic_debug_info(
                                |ctx| ctx.codegen_static(def_id, *item),
                                format!("codegen_static: {def_id:?}"),
                                def_id,
                            );
                        }
                        MonoItem::GlobalAsm(_) => {} // We have already warned above
                    }
                }
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

        // No output should be generated if user selected no_codegen.
        if !tcx.sess.opts.unstable_opts.no_codegen && tcx.sess.opts.output_types.should_codegen() {
            let pretty = self.queries.lock().unwrap().get_output_pretty_json();
            write_file(&symtab_goto, ArtifactType::PrettyNameMap, &pretty_name_map, pretty);
            if gcx.queries.get_write_json_symtab() {
                write_file(&symtab_goto, ArtifactType::SymTab, &gcx.symbol_table, pretty);
                symbol_table_to_gotoc(&tcx, &symtab_goto);
            } else {
                write_goto_binary_file(symtab_goto, &gcx.symbol_table);
            }
            write_file(&symtab_goto, ArtifactType::TypeMap, &type_map, pretty);
            // If they exist, write out vtable virtual call function pointer restrictions
            if let Some(restrictions) = vtable_restrictions {
                write_file(&symtab_goto, ArtifactType::VTableRestriction, &restrictions, pretty);
            }
        }

        (gcx, items)
    }
}

impl CodegenBackend for GotocCodegenBackend {
    fn metadata_loader(&self) -> Box<MetadataLoaderDyn> {
        Box::new(rustc_codegen_ssa::back::metadata::DefaultMetadataLoader)
    }

    fn provide(&self, providers: &mut Providers) {
        provide::provide(providers, &self.queries.lock().unwrap());
    }

    fn provide_extern(&self, providers: &mut ty::query::ExternProviders) {
        provide::provide_extern(providers);
    }

    fn print_version(&self) {
        println!("Kani-goto version: {}", env!("CARGO_PKG_VERSION"));
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
        super::utils::init();

        // Queries shouldn't change today once codegen starts.
        let queries = self.queries.lock().unwrap().clone();
        check_target(tcx.sess);
        check_options(tcx.sess);
        check_crate_items(tcx, queries.get_ignore_global_asm());

        // Codegen all items that need to be processed according to the selected reachability mode:
        //
        // - Harnesses: Generate one model per local harnesses (marked with `kani::proof` attribute).
        // - Tests: Generate one model per test harnesses.
        // - PubFns: Generate code for all reachable logic starting from the local public functions.
        // - None: Don't generate code. This is used to compile dependencies.
        let base_filename = tcx.output_filenames(()).output_path(OutputType::Object);
        let reachability = queries.get_reachability_analysis();
        let mut results = GotoCodegenResults::new(tcx, reachability);
        match reachability {
            ReachabilityType::Harnesses => {
                // Cross-crate collecting of all items that are reachable from the crate harnesses.
                let harnesses = filter_crate_items(tcx, |_, def_id| is_proof_harness(tcx, def_id));
                for harness in harnesses {
                    let metadata = gen_proof_metadata(tcx, harness.def_id(), &base_filename);
                    let model_path = &metadata.goto_file.as_ref().unwrap();
                    let (gcx, items) =
                        self.codegen_items(tcx, &[harness], model_path, &results.machine_model);
                    results.extend(gcx, items, Some(metadata));
                }
            }
            ReachabilityType::Tests => {
                // We're iterating over crate items here, so what we have to codegen is the "test description" containing the
                // test closure that we want to execute
                // TODO: Refactor this code so we can guarantee that the pair (test_fn, test_desc) actually match.
                let mut descriptions = vec![];
                let harnesses = filter_const_crate_items(tcx, |_, def_id| {
                    if is_test_harness_description(tcx, def_id) {
                        descriptions.push(def_id);
                        true
                    } else {
                        false
                    }
                });
                // Codegen still takes a considerable amount, thus, we only generate one model for
                // all harnesses and copy them for each harness.
                // We will be able to remove this once we optimize all calls to CBMC utilities.
                // https://github.com/model-checking/kani/issues/1971
                let model_path = base_filename.with_extension(ArtifactType::SymTabGoto);
                let (gcx, items) =
                    self.codegen_items(tcx, &harnesses, &model_path, &results.machine_model);
                results.extend(gcx, items, None);

                for (test_fn, test_desc) in harnesses.iter().zip(descriptions.iter()) {
                    let instance =
                        if let MonoItem::Fn(instance) = test_fn { instance } else { continue };
                    let metadata = gen_test_metadata(tcx, *test_desc, *instance, &base_filename);
                    let test_model_path = &metadata.goto_file.as_ref().unwrap();
                    std::fs::copy(&model_path, &test_model_path).expect(&format!(
                        "Failed to copy {} to {}",
                        model_path.display(),
                        test_model_path.display()
                    ));
                    results.harnesses.push(metadata);
                }
            }
            ReachabilityType::None => {}
            ReachabilityType::PubFns => {
                let entry_fn = tcx.entry_fn(()).map(|(id, _)| id);
                let local_reachable = filter_crate_items(tcx, |_, def_id| {
                    (tcx.is_reachable_non_generic(def_id) && tcx.def_kind(def_id).is_fn_like())
                        || entry_fn == Some(def_id)
                });
                let model_path = base_filename.with_extension(ArtifactType::SymTabGoto);
                let (gcx, items) =
                    self.codegen_items(tcx, &local_reachable, &model_path, &results.machine_model);
                results.extend(gcx, items, None);
            }
        }

        if reachability != ReachabilityType::None {
            // Print compilation report.
            results.print_report(tcx);

            // In a workspace, cargo seems to be using the same file prefix to build a crate that is
            // a package lib and also a dependency of another package.
            // To avoid overriding the metadata for its verification, we skip this step when
            // reachability is None, even because there is nothing to record.
            write_file(
                &base_filename,
                ArtifactType::Metadata,
                &results.generate_metadata(),
                queries.get_output_pretty_json(),
            );
        }
        codegen_results(tcx, rustc_metadata, &results.machine_model)
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
        let requested_crate_types = sess.crate_types();
        for crate_type in requested_crate_types {
            let out_path = out_filename(
                sess,
                *crate_type,
                outputs,
                codegen_results.crate_info.local_crate_name,
            );
            debug!(?crate_type, ?out_path, "link");
            if *crate_type == CrateType::Rlib {
                // Emit the `rlib` that contains just one file: `<crate>.rmeta`
                let mut builder = Box::new(ArArchiveBuilder::new(sess, get_native_object_symbols));
                let tmp_dir = TempFileBuilder::new().prefix("kani").tempdir().unwrap();
                let path = MaybeTempDir::new(tmp_dir, sess.opts.cg.save_temps);
                let (metadata, _metadata_position) = create_wrapper_file(
                    sess,
                    b".rmeta".to_vec(),
                    codegen_results.metadata.raw_data(),
                );
                let metadata = emit_wrapper_file(sess, &metadata, &path, METADATA_FILENAME);
                builder.add_file(&metadata);
                builder.build(&out_path);
            } else {
                // Write the location of the kani metadata file in the requested compiler output file.
                let base_filename = outputs.output_path(OutputType::Object);
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

fn check_target(session: &Session) {
    // The requirement below is needed to build a valid CBMC machine model
    // in function `machine_model_from_session` from
    // src/kani-compiler/src/codegen_cprover_gotoc/context/goto_ctx.rs
    let is_linux_target = session.target.llvm_target == "x86_64-unknown-linux-gnu";
    // Comparison with `x86_64-apple-darwin` does not work well because the LLVM
    // target may become `x86_64-apple-macosx10.7.0` (or similar) and fail
    let is_x86_64_darwin_target = session.target.llvm_target.starts_with("x86_64-apple-");
    // looking for `arm64-apple-*`
    let is_arm64_darwin_target = session.target.llvm_target.starts_with("arm64-apple-");

    if !is_linux_target && !is_x86_64_darwin_target && !is_arm64_darwin_target {
        let err_msg = format!(
            "Kani requires the target platform to be `x86_64-unknown-linux-gnu` or \
            `x86_64-apple-*` or `arm64-apple-*`, but it is {}",
            &session.target.llvm_target
        );
        session.err(&err_msg);
    }

    session.abort_if_errors();
}

fn check_options(session: &Session) {
    // The requirements for `min_global_align` and `endian` are needed to build
    // a valid CBMC machine model in function `machine_model_from_session` from
    // src/kani-compiler/src/codegen_cprover_gotoc/context/goto_ctx.rs
    match session.target.options.min_global_align {
        Some(1) => (),
        Some(align) => {
            let err_msg = format!(
                "Kani requires the target architecture option `min_global_align` to be 1, but it is {align}."
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

    session.abort_if_errors();
}

/// Return a struct that contains information about the codegen results as expected by `rustc`.
fn codegen_results(
    tcx: TyCtxt,
    rustc_metadata: EncodedMetadata,
    machine: &MachineModel,
) -> Box<dyn Any> {
    let work_products = FxHashMap::<WorkProductId, WorkProduct>::default();
    Box::new((
        CodegenResults {
            modules: vec![],
            allocator_module: None,
            metadata_module: None,
            metadata: rustc_metadata,
            crate_info: CrateInfo::new(tcx, machine.architecture.clone()),
        },
        work_products,
    ))
}

fn symbol_table_to_gotoc(tcx: &TyCtxt, base_path: &Path) -> PathBuf {
    let output_filename = base_path.to_path_buf();
    let input_filename = convert_type(base_path, ArtifactType::SymTabGoto, ArtifactType::SymTab);

    let args = vec![
        input_filename.clone().into_os_string(),
        "--out".into(),
        OsString::from(output_filename.as_os_str()),
    ];
    // TODO get symtab2gb path from self
    let mut cmd = Command::new("symtab2gb");
    cmd.args(args);
    info!("[Kani] Running: `{:?} {:?}`", cmd.get_program(), cmd.get_args());

    let result = with_timer(
        || {
            cmd.output()
                .expect(&format!("Failed to generate goto model for {}", input_filename.display()))
        },
        "symtab2gb",
    );
    if !result.status.success() {
        error!("Symtab error output:\n{}", String::from_utf8_lossy(&result.stderr));
        error!("Symtab output:\n{}", String::from_utf8_lossy(&result.stdout));
        let err_msg = format!(
            "Failed to generate goto model:\n\tsymtab2gb failed on file {}.",
            input_filename.display()
        );
        tcx.sess.err(&err_msg);
        tcx.sess.abort_if_errors();
    };
    output_filename
}

/// Print MIR for the reachable items if the `--emit mir` option was provided to rustc.
fn dump_mir_items(tcx: TyCtxt, items: &[MonoItem]) {
    /// Convert MonoItem into a DefId.
    /// Skip stuff that we cannot generate the MIR items.
    fn visible_item<'tcx>(item: &MonoItem<'tcx>) -> Option<(MonoItem<'tcx>, DefId)> {
        match item {
            // Exclude FnShims and others that cannot be dumped.
            MonoItem::Fn(instance) if matches!(instance.def, InstanceDef::Item(..)) => {
                Some((*item, instance.def_id()))
            }
            MonoItem::Fn(..) => None,
            MonoItem::Static(def_id) => Some((*item, *def_id)),
            MonoItem::GlobalAsm(_) => None,
        }
    }

    if tcx.sess.opts.output_types.contains_key(&OutputType::Mir) {
        // Create output buffer.
        let outputs = tcx.output_filenames(());
        let path = outputs.output_path(OutputType::Mir).with_extension("kani.mir");
        let out_file = File::create(&path).unwrap();
        let mut writer = BufWriter::new(out_file);

        // For each def_id, dump their MIR
        for (item, def_id) in items.iter().filter_map(visible_item) {
            writeln!(writer, "// Item: {item:?}").unwrap();
            write_mir_pretty(tcx, Some(def_id), &mut writer).unwrap();
        }
    }
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

struct GotoCodegenResults<'tcx> {
    reachability: ReachabilityType,
    harnesses: Vec<HarnessMetadata>,
    unsupported_constructs: UnsupportedConstructs,
    concurrent_constructs: UnsupportedConstructs,
    items: Vec<MonoItem<'tcx>>,
    crate_name: InternedString,
    machine_model: MachineModel,
}

impl<'tcx> GotoCodegenResults<'tcx> {
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
        }
    }

    fn extend(
        &mut self,
        gcx: GotocCtx,
        items: Vec<MonoItem<'tcx>>,
        metadata: Option<HarnessMetadata>,
    ) {
        let mut items = items;
        self.harnesses.extend(metadata.into_iter());
        self.concurrent_constructs.extend(gcx.concurrent_constructs.into_iter());
        self.unsupported_constructs.extend(gcx.unsupported_constructs.into_iter());
        self.items.append(&mut items);
    }

    /// Prints a report at the end of the compilation.
    fn print_report(&self, tcx: TyCtxt<'tcx>) {
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
            tcx.sess.warn(&msg);
        }

        if !self.concurrent_constructs.is_empty() {
            let mut msg = String::from(
                "Kani currently does not support concurrency. The following constructs will be treated \
                as sequential operations:\n",
            );
            for (construct, locations) in self.concurrent_constructs.iter() {
                writeln!(&mut msg, "    - {construct} ({})", locations.len()).unwrap();
            }
            tcx.sess.warn(&msg);
        }

        // Print some compilation stats.
        if tracing::enabled!(tracing::Level::INFO) {
            analysis::print_stats(tcx, &self.items);
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
    let pointer_width = sess.target.pointer_width.into();

    // The model assumes the following values for session options:
    //   * `min_global_align`: 1
    //   * `endian`: `Endian::Little`
    //
    // We check these options in function `check_options` from
    // src/kani-compiler/src/codegen_cprover_gotoc/compiler_interface.rs
    // and error if their values are not the ones we expect.
    let alignment = sess.target.options.min_global_align.unwrap_or(1);
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
            let long_double_width = 64;
            let long_int_width = 64;
            let long_long_int_width = 64;
            let short_int_width = 16;
            let single_width = 32;
            let wchar_t_is_unsigned = false;
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
