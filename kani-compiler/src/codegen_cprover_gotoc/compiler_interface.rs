// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code necessary to interface with the compiler backend

use crate::codegen_cprover_gotoc::GotocCtx;
use crate::kani_middle::provide;
use crate::kani_middle::reachability::{
    collect_reachable_items, filter_closures_in_const_crate_items, filter_crate_items,
};
use bitflags::_core::any::Any;
use cbmc::goto_program::Location;
use cbmc::{InternedString, MachineModel};
use kani_metadata::{ArtifactType, HarnessMetadata, KaniMetadata};
use kani_queries::{QueryDb, ReachabilityType, UserInput};
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_codegen_ssa::{CodegenResults, CrateInfo};
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::ErrorGuaranteed;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_metadata::EncodedMetadata;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::mir::mono::{CodegenUnit, MonoItem};
use rustc_middle::mir::write_mir_pretty;
use rustc_middle::ty::query::Providers;
use rustc_middle::ty::{self, InstanceDef, TyCtxt};
use rustc_session::config::{OutputFilenames, OutputType};
use rustc_session::cstore::MetadataLoaderDyn;
use rustc_session::Session;
use rustc_span::def_id::DefId;
use rustc_target::abi::Endian;
use rustc_target::spec::PanicStrategy;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write as IoWrite;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct GotocCodegenBackend {
    queries: Rc<QueryDb>,
}

impl GotocCodegenBackend {
    pub fn new(queries: &Rc<QueryDb>) -> Self {
        GotocCodegenBackend { queries: Rc::clone(queries) }
    }
}

impl CodegenBackend for GotocCodegenBackend {
    fn metadata_loader(&self) -> Box<MetadataLoaderDyn> {
        Box::new(rustc_codegen_ssa::back::metadata::DefaultMetadataLoader)
    }

    fn provide(&self, providers: &mut Providers) {
        provide::provide(providers, &self.queries);
    }

    fn provide_extern(&self, providers: &mut ty::query::ExternProviders) {
        provide::provide_extern(providers);
    }

    fn codegen_crate(
        &self,
        tcx: TyCtxt,
        rustc_metadata: EncodedMetadata,
        need_metadata_module: bool,
    ) -> Box<dyn Any> {
        super::utils::init();

        // Follow rustc naming convention (cx is abbrev for context).
        // https://rustc-dev-guide.rust-lang.org/conventions.html#naming-conventions
        let mut gcx = GotocCtx::new(tcx, self.queries.clone());
        check_target(tcx.sess);
        check_options(tcx.sess, need_metadata_module);
        check_crate_items(&gcx);

        let items = with_timer(|| collect_codegen_items(&gcx), "codegen reachability analysis");
        if items.is_empty() {
            // There's nothing to do.
            return codegen_results(tcx, rustc_metadata, gcx.symbol_table.machine_model());
        }
        dump_mir_items(tcx, &items);

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
                for item in items {
                    match item {
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
                                |ctx| ctx.codegen_static(def_id, item),
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

        // Print compilation report.
        print_report(&gcx, tcx);

        // Map MIR types to GotoC types
        let type_map: BTreeMap<InternedString, InternedString> =
            BTreeMap::from_iter(gcx.type_map.iter().map(|(k, v)| (*k, v.to_string().into())));

        // Get the vtable function pointer restrictions if requested
        let vtable_restrictions = if gcx.vtable_ctx.emit_vtable_restrictions {
            Some(gcx.vtable_ctx.get_virtual_function_restrictions())
        } else {
            None
        };

        let metadata = generate_metadata(&gcx, tcx);

        // No output should be generated if user selected no_codegen.
        if !tcx.sess.opts.unstable_opts.no_codegen && tcx.sess.opts.output_types.should_codegen() {
            let outputs = tcx.output_filenames(());
            let base_filename = outputs.output_path(OutputType::Object);
            let pretty = self.queries.get_output_pretty_json();
            write_file(&base_filename, ArtifactType::SymTab, &gcx.symbol_table, pretty);
            write_file(&base_filename, ArtifactType::TypeMap, &type_map, pretty);
            write_file(&base_filename, ArtifactType::Metadata, &metadata, pretty);
            // If they exist, write out vtable virtual call function pointer restrictions
            if let Some(restrictions) = vtable_restrictions {
                write_file(&base_filename, ArtifactType::VTableRestriction, &restrictions, pretty);
            }
            symbol_table_to_gotoc(&tcx, &base_filename);
        }
        codegen_results(tcx, rustc_metadata, gcx.symbol_table.machine_model())
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
        sess: &Session,
        codegen_results: CodegenResults,
        outputs: &OutputFilenames,
    ) -> Result<(), ErrorGuaranteed> {
        // In `link`, we need to do two things:
        // 1. We need to emit `rlib` files normally. Cargo expects these in some circumstances and sends
        //    them to subsequent builds with `-L`.
        // 2. We MUST NOT try to invoke the native linker, because that will fail. We don't have real objects.
        // This is normally not a problem: usually we only get one requested `crate-type`.
        // But let's be careful and fail loudly if we get conflicting requests:
        let requested_crate_types = sess.crate_types();
        // Quit successfully if we don't need an `rlib`:
        if !requested_crate_types.contains(&rustc_session::config::CrateType::Rlib) {
            return Ok(());
        }
        // Fail loudly if we need an `rlib` (above!) and *also* an executable, which
        // we can't produce, and can't easily suppress in `link_binary`:
        if requested_crate_types.contains(&rustc_session::config::CrateType::Executable) {
            sess.err("Build crate-type requested both rlib and executable, and Kani cannot handle this situation");
            sess.abort_if_errors();
        }

        // All this ultimately boils down to is emitting an `rlib` that contains just one file: `lib.rmeta`
        use rustc_codegen_ssa::back::link::link_binary;
        link_binary(
            sess,
            &crate::codegen_cprover_gotoc::archive::ArArchiveBuilderBuilder,
            &codegen_results,
            outputs,
        )
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

/// Check that all crate items are supported and there's no misconfiguration.
/// This method will exhaustively print any error / warning and it will abort at the end if any
/// error was found.
fn check_crate_items(gcx: &GotocCtx) {
    let tcx = gcx.tcx;
    for item in tcx.hir_crate_items(()).items() {
        let def_id = item.owner_id.def_id.to_def_id();
        gcx.check_attributes(def_id);
        if tcx.def_kind(def_id) == DefKind::GlobalAsm {
            if !gcx.queries.get_ignore_global_asm() {
                let error_msg = format!(
                    "Crate {} contains global ASM, which is not supported by Kani. Rerun with \
                    `--enable-unstable --ignore-global-asm` to suppress this error \
                    (**Verification results may be impacted**).",
                    gcx.short_crate_name()
                );
                tcx.sess.err(&error_msg);
            } else {
                warn!(
                    "Ignoring global ASM in crate {}. Verification results may be impacted.",
                    gcx.short_crate_name()
                );
            }
        }
    }
    tcx.sess.abort_if_errors();
}

/// Prints a report at the end of the compilation.
fn print_report<'tcx>(ctx: &GotocCtx, tcx: TyCtxt<'tcx>) {
    // Print all unsupported constructs.
    if !ctx.unsupported_constructs.is_empty() {
        // Sort alphabetically.
        let unsupported: BTreeMap<String, &Vec<Location>> = ctx
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

/// Retrieve all items that need to be processed according to the selected reachability mode:
///
/// - Harnesses: Cross-crate collection of all reachable items starting from local harnesses.
/// - None: Skip collection and codegen all together. This is used to compile dependencies.
/// - Legacy: Use regular compiler collection that will collect local items, and a few cross
/// crate items (such as generic functions and functions candidate to be inlined).
/// - PubFns: Cross-crate reachability analysis that use the local public fns as starting point.
/// - Tests: Cross-crate collection of all reachable items starting from test harnesses.
fn collect_codegen_items<'tcx>(gcx: &GotocCtx<'tcx>) -> Vec<MonoItem<'tcx>> {
    let tcx = gcx.tcx;
    let reach = gcx.queries.get_reachability_analysis();
    debug!(?reach, "starting_points");
    match reach {
        ReachabilityType::Legacy => {
            // Use rustc monomorphizer to retrieve items to codegen.
            let codegen_units: &'tcx [CodegenUnit<'_>] = tcx.collect_and_partition_mono_items(()).1;
            codegen_units
                .iter()
                .flat_map(|cgu| cgu.items_in_deterministic_order(tcx))
                .map(|(item, _)| item)
                .collect()
        }
        ReachabilityType::Harnesses => {
            // Cross-crate collecting of all items that are reachable from the crate harnesses.
            let harnesses = filter_crate_items(tcx, |_, def_id| gcx.is_proof_harness(def_id));
            collect_reachable_items(tcx, &harnesses).into_iter().collect()
        }
        ReachabilityType::Tests => {
            // We're iterating over crate items here, so what we have to codegen is the "test description" containing the
            // test closure that we want to execute
            let harnesses = filter_closures_in_const_crate_items(tcx, |_, def_id| {
                gcx.is_test_harness_description(def_id)
            });
            collect_reachable_items(tcx, &harnesses).into_iter().collect()
        }
        ReachabilityType::None => Vec::new(),
        ReachabilityType::PubFns => {
            let entry_fn = tcx.entry_fn(()).map(|(id, _)| id);
            let local_reachable = filter_crate_items(tcx, |_, def_id| {
                tcx.is_reachable_non_generic(def_id) || entry_fn == Some(def_id)
            });
            collect_reachable_items(tcx, &local_reachable).into_iter().collect()
        }
    }
}

fn symbol_table_to_gotoc(tcx: &TyCtxt, file: &Path) -> PathBuf {
    let output_filename = file.with_extension(ArtifactType::SymTabGoto);
    let input_filename = file.with_extension(ArtifactType::SymTab);

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
            MonoItem::Fn(instance)
                if matches!(
                    instance.def,
                    InstanceDef::FnPtrShim(..) | InstanceDef::ClosureOnceShim { .. }
                ) =>
            {
                None
            }
            MonoItem::Fn(instance) => Some((*item, instance.def_id())),
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

/// Method that generates `KaniMetadata` from the given compilation context.
/// This is a temporary method used until we generate a model per-harness.
/// See <https://github.com/model-checking/kani/issues/1855> for more details.
fn generate_metadata(gcx: &GotocCtx, tcx: TyCtxt) -> KaniMetadata {
    let outputs = tcx.output_filenames(());
    let model_file =
        outputs.output_path(OutputType::Object).with_extension(ArtifactType::SymTabGoto);
    let extend_harnesses = |mut harnesses: Vec<HarnessMetadata>| {
        for harness in harnesses.iter_mut() {
            harness.goto_file = Some(model_file.clone());
        }
        harnesses
    };
    KaniMetadata {
        crate_name: tcx.crate_name(LOCAL_CRATE).to_string(),
        proof_harnesses: extend_harnesses(gcx.proof_harnesses.clone()),
        unsupported_features: gcx.unsupported_metadata(),
        test_harnesses: extend_harnesses(gcx.test_harnesses.clone()),
    }
}

pub fn write_file<T>(base_path: &Path, file_type: ArtifactType, source: &T, pretty: bool)
where
    T: serde::Serialize,
{
    let filename = base_path.with_extension(file_type);
    debug!(?filename, "write_json");
    let out_file = File::create(&filename).unwrap();
    let writer = BufWriter::new(out_file);
    if pretty {
        serde_json::to_writer_pretty(writer, &source).unwrap();
    } else {
        serde_json::to_writer(writer, &source).unwrap();
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
