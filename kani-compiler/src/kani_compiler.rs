// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module defines all compiler extensions that form the Kani compiler.
//!
//! The [KaniCompiler] can be used across multiple rustc driver runs ([RunCompiler::run()]),
//! which is used to implement stubs.
//!
//! In the first run, [KaniCompiler::config] will implement the compiler configuration and it will
//! also collect any stubs that may need to be applied. This method will be a no-op for any
//! subsequent runs. The [KaniCompiler] will parse options that are passed via `-C llvm-args`.
//!
//! If no stubs need to be applied, the compiler will proceed to generate goto code, and it won't
//! need any extra runs. However, if stubs are required, we will have to restart the rustc driver
//! in order to apply the stubs. For the subsequent runs, we add the stub configuration to
//! `-C llvm-args`.

#[cfg(feature = "cprover")]
use crate::codegen_cprover_gotoc::GotocCodegenBackend;
use crate::kani_middle::attributes::is_proof_harness;
use crate::kani_middle::check_crate_items;
use crate::kani_middle::metadata::gen_proof_metadata;
use crate::kani_middle::reachability::filter_crate_items;
use crate::kani_middle::stubbing::{self, harness_stub_map};
use crate::kani_queries::{QueryDb, ReachabilityType};
use crate::parser::{self, KaniCompilerParser};
use crate::session::init_session;
use kani_metadata::{ArtifactType, HarnessMetadata, KaniMetadata};
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::definitions::DefPathHash;
use rustc_interface::Config;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::{ErrorOutputType, OutputType};
use rustc_session::EarlyErrorHandler;
use rustc_span::ErrorGuaranteed;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::BufWriter;
use std::mem;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use tracing::debug;

/// Run the Kani flavour of the compiler.
/// This may require multiple runs of the rustc driver ([RunCompiler::run]).
pub fn run(args: Vec<String>) -> ExitCode {
    let mut kani_compiler = KaniCompiler::new();
    match kani_compiler.run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// Configure the cprover backend that generate goto-programs.
#[cfg(feature = "cprover")]
fn backend(queries: Arc<Mutex<QueryDb>>) -> Box<dyn CodegenBackend> {
    Box::new(GotocCodegenBackend::new(queries))
}

/// Fallback backend. It will trigger an error if no backend has been enabled.
#[cfg(not(feature = "cprover"))]
fn backend(queries: Arc<Mutex<QueryDb>>) -> Box<CodegenBackend> {
    compile_error!("No backend is available. Only supported value today is `cprover`");
}

/// A stable (across compilation sessions) identifier for the harness function.
type HarnessId = DefPathHash;

/// A set of stubs.
type Stubs = BTreeMap<DefPathHash, DefPathHash>;

#[derive(Debug)]
struct HarnessInfo {
    pub metadata: HarnessMetadata,
    pub stub_map: Stubs,
}

/// Represents the current compilation stage.
///
/// The Kani compiler may run the Rust compiler multiple times since stubbing has to be applied
/// to the entire Rust compiler session.
///
/// - We always start in the [CompilationStage::Init].
/// - After [CompilationStage::Init] we transition to either
///   - [CompilationStage::CodegenNoStubs] on a regular crate compilation, this will follow Init.
///   - [CompilationStage::Done], running the compiler to gather information, such as `--version`
///     will skip code generation completely, and there is no work to be done.
/// - After the [CompilationStage::CodegenNoStubs], we transition to either
///   - [CompilationStage::CodegenWithStubs] when there is at least one harness with stubs.
///   - [CompilationStage::Done] where there is no harness left to process.
/// - The [CompilationStage::CodegenWithStubs] can last multiple Rust compiler runs. Once there is
///   no harness left, we move to [CompilationStage::Done].
/// I.e.:
/// ```dot
/// graph CompilationStage {
///   Init -> {CodegenNoStubs, Done}
///   CodegenNoStubs -> {CodegenStubs, Done}
///   // Loop up to N harnesses times.
///   CodegenStubs -> {CodegenStubs, Done}
///   Done
/// }
/// ```
#[allow(dead_code)]
#[derive(Debug)]
enum CompilationStage {
    /// Initial state that the compiler is always instantiated with.
    /// In this stage, we initialize the Query and collect all harnesses.
    Init,
    /// Stage where the compiler will perform codegen of all harnesses that don't use stub.
    CodegenNoStubs {
        target_harnesses: Vec<HarnessId>,
        next_harnesses: Vec<Vec<HarnessId>>,
        all_harnesses: HashMap<HarnessId, HarnessInfo>,
    },
    /// Stage where the compiler will codegen harnesses that use stub, one group at a time.
    /// The harnesses at this stage are grouped according to the stubs they are using. For now,
    /// we ensure they have the exact same set of stubs.
    CodegenWithStubs {
        target_harnesses: Vec<HarnessId>,
        next_harnesses: Vec<Vec<HarnessId>>,
        all_harnesses: HashMap<HarnessId, HarnessInfo>,
    },
    Done,
}

impl CompilationStage {
    pub fn is_init(&self) -> bool {
        matches!(self, CompilationStage::Init)
    }

    pub fn is_done(&self) -> bool {
        matches!(self, CompilationStage::Done)
    }
}

/// This object controls the compiler behavior.
///
/// It is responsible for initializing the query database, as well as controlling the compiler
/// state machine. For stubbing, we may require multiple iterations of the rustc driver, which is
/// controlled and configured via KaniCompiler.
struct KaniCompiler {
    /// Store the queries database. The queries should be initialized as part of `config` when the
    /// compiler state is Init.
    /// Note that we need to share the queries with the backend before `config` is called.
    pub queries: Arc<Mutex<QueryDb>>,
    /// The state which the compiler is at.
    stage: CompilationStage,
}

impl KaniCompiler {
    /// Create a new [KaniCompiler] instance.
    pub fn new() -> KaniCompiler {
        KaniCompiler { queries: QueryDb::new(), stage: CompilationStage::Init }
    }

    /// Compile the current crate with the given arguments.
    ///
    /// Since harnesses may have different attributes that affect compilation, Kani compiler can
    /// actually invoke the rust compiler multiple times.
    pub fn run(&mut self, orig_args: Vec<String>) -> Result<(), ErrorGuaranteed> {
        while !self.stage.is_done() {
            debug!(next=?self.stage, "run");
            match &self.stage {
                CompilationStage::Init => {
                    self.run_compilation_session(&orig_args)?;
                }
                CompilationStage::CodegenNoStubs { .. } => {
                    unreachable!("This stage should always run in the same session as Init");
                }
                CompilationStage::CodegenWithStubs { target_harnesses, all_harnesses, .. } => {
                    assert!(!target_harnesses.is_empty(), "expected at least one target harness");
                    let target_harness = &target_harnesses[0];
                    let extra_arg =
                        stubbing::mk_rustc_arg(&all_harnesses[&target_harness].stub_map);
                    let mut args = orig_args.clone();
                    args.push(extra_arg);
                    self.run_compilation_session(&args)?;
                }
                CompilationStage::Done => {
                    unreachable!("There's nothing to be done here.")
                }
            };

            self.next_stage();
        }
        Ok(())
    }

    /// Set up the next compilation stage after a `rustc` run.
    fn next_stage(&mut self) {
        self.stage = match &mut self.stage {
            CompilationStage::Init => {
                // This may occur when user passes arguments like --version or --help.
                CompilationStage::Done
            }
            CompilationStage::CodegenNoStubs { next_harnesses, all_harnesses, .. }
            | CompilationStage::CodegenWithStubs { next_harnesses, all_harnesses, .. } => {
                if let Some(target_harnesses) = next_harnesses.pop() {
                    assert!(!target_harnesses.is_empty(), "expected at least one target harness");
                    CompilationStage::CodegenWithStubs {
                        target_harnesses,
                        next_harnesses: mem::take(next_harnesses),
                        all_harnesses: mem::take(all_harnesses),
                    }
                } else {
                    CompilationStage::Done
                }
            }
            CompilationStage::Done => {
                unreachable!()
            }
        };
    }

    /// Run the Rust compiler with the given arguments and pass `&mut self` to handle callbacks.
    fn run_compilation_session(&mut self, args: &[String]) -> Result<(), ErrorGuaranteed> {
        debug!(?args, "run_compilation_session");
        let queries = self.queries.clone();
        let mut compiler = RunCompiler::new(args, self);
        compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| backend(queries))));
        compiler.run()
    }

    /// Gather and process all harnesses from this crate that shall be compiled.
    fn process_harnesses(&self, tcx: TyCtxt) -> CompilationStage {
        if self.queries.lock().unwrap().reachability_analysis == ReachabilityType::Harnesses {
            let base_filename = tcx.output_filenames(()).output_path(OutputType::Object);
            let harnesses = filter_crate_items(tcx, |_, def_id| is_proof_harness(tcx, def_id));
            let all_harnesses = harnesses
                .into_iter()
                .map(|harness| {
                    let def_id = harness.def_id();
                    let def_path = tcx.def_path_hash(def_id);
                    let metadata = gen_proof_metadata(tcx, def_id, &base_filename);
                    let stub_map = harness_stub_map(tcx, def_id, &metadata);
                    (def_path, HarnessInfo { metadata, stub_map })
                })
                .collect::<HashMap<_, _>>();

            let (no_stubs, with_stubs): (Vec<_>, Vec<_>) =
                if self.queries.lock().unwrap().stubbing_enabled {
                    // Partition harnesses that don't have stub with the ones with stub.
                    all_harnesses
                        .keys()
                        .cloned()
                        .partition(|harness| all_harnesses[harness].stub_map.is_empty())
                } else {
                    // Generate code without stubs.
                    (all_harnesses.keys().cloned().collect(), vec![])
                };
            // Store metadata file.
            self.store_metadata(tcx, &all_harnesses);

            // Even if no_stubs is empty we still need to store metadata.
            CompilationStage::CodegenNoStubs {
                target_harnesses: no_stubs,
                next_harnesses: group_by_stubs(with_stubs, &all_harnesses),
                all_harnesses,
            }
        } else {
            // Leave other reachability type handling as is for now.
            CompilationStage::CodegenNoStubs {
                target_harnesses: vec![],
                next_harnesses: vec![],
                all_harnesses: HashMap::default(),
            }
        }
    }

    /// Prepare the query for the next codegen stage.
    fn prepare_codegen(&mut self) -> Compilation {
        debug!(stage=?self.stage, "prepare_codegen");
        match &self.stage {
            CompilationStage::CodegenNoStubs { target_harnesses, all_harnesses, .. }
            | CompilationStage::CodegenWithStubs { target_harnesses, all_harnesses, .. } => {
                debug!(
                    harnesses=?target_harnesses
                        .iter()
                        .map(|h| &all_harnesses[h].metadata.pretty_name)
                        .collect::<Vec<_>>(),
                        "prepare_codegen"
                );
                let queries = &mut (*self.queries.lock().unwrap());
                queries.harnesses_info = target_harnesses
                    .iter()
                    .map(|harness| {
                        (*harness, all_harnesses[harness].metadata.goto_file.clone().unwrap())
                    })
                    .collect();
                Compilation::Continue
            }
            CompilationStage::Init | CompilationStage::Done => unreachable!(),
        }
    }

    /// Write the metadata to a file
    fn store_metadata(&self, tcx: TyCtxt, all_harnesses: &HashMap<HarnessId, HarnessInfo>) {
        let (proof_harnesses, test_harnesses) = all_harnesses
            .values()
            .map(|info| &info.metadata)
            .cloned()
            .partition(|md| md.attributes.proof);
        let metadata = KaniMetadata {
            crate_name: tcx.crate_name(LOCAL_CRATE).as_str().into(),
            proof_harnesses,
            unsupported_features: vec![],
            test_harnesses,
        };
        let mut filename = tcx.output_filenames(()).output_path(OutputType::Object);
        filename.set_extension(ArtifactType::Metadata);
        debug!(?filename, "write_metadata");
        let out_file = File::create(&filename).unwrap();
        let writer = BufWriter::new(out_file);
        if self.queries.lock().unwrap().output_pretty_json {
            serde_json::to_writer_pretty(writer, &metadata).unwrap();
        } else {
            serde_json::to_writer(writer, &metadata).unwrap();
        }
    }
}

/// Group the harnesses by their stubs.
fn group_by_stubs(
    harnesses: Vec<HarnessId>,
    all_harnesses: &HashMap<HarnessId, HarnessInfo>,
) -> Vec<Vec<HarnessId>> {
    let mut per_stubs: BTreeMap<&Stubs, Vec<HarnessId>> = BTreeMap::default();
    for harness in harnesses {
        per_stubs.entry(&all_harnesses[&harness].stub_map).or_default().push(harness)
    }
    per_stubs.into_values().collect()
}

/// Use default function implementations.
impl Callbacks for KaniCompiler {
    /// Configure the [KaniCompiler] `self` object during the [CompilationStage::Init].
    fn config(&mut self, config: &mut Config) {
        if self.stage.is_init() {
            let mut args = vec!["kani-compiler".to_string()];
            args.extend(config.opts.cg.llvm_args.iter().cloned());
            let matches = parser::parser().get_matches_from(&args);
            init_session(
                &matches,
                matches!(config.opts.error_format, ErrorOutputType::Json { .. }),
            );
            // Configure queries.
            let queries = &mut (*self.queries.lock().unwrap());
            queries.emit_vtable_restrictions = matches.get_flag(parser::RESTRICT_FN_PTRS);
            queries.check_assertion_reachability = matches.get_flag(parser::ASSERTION_REACH_CHECKS);
            queries.output_pretty_json = matches.get_flag(parser::PRETTY_OUTPUT_FILES);
            queries.ignore_global_asm = matches.get_flag(parser::IGNORE_GLOBAL_ASM);
            queries.write_json_symtab =
                cfg!(feature = "write_json_symtab") || matches.get_flag(parser::WRITE_JSON_SYMTAB);
            queries.reachability_analysis = matches.reachability_type();

            if let Some(features) = matches.get_many::<String>(parser::UNSTABLE_FEATURE) {
                queries.unstable_features = features.cloned().collect::<Vec<_>>();
            }

            if matches.get_flag(parser::ENABLE_STUBBING)
                && queries.reachability_analysis == ReachabilityType::Harnesses
            {
                queries.stubbing_enabled = true;
            }
            debug!(?queries, "config end");
        }
    }

    /// During the initialization state, we collect the crate harnesses and prepare for codegen.
    fn after_analysis<'tcx>(
        &mut self,
        _handler: &EarlyErrorHandler,
        _compiler: &rustc_interface::interface::Compiler,
        rustc_queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        if self.stage.is_init() {
            self.stage = rustc_queries.global_ctxt().unwrap().enter(|tcx| {
                check_crate_items(tcx, self.queries.lock().unwrap().ignore_global_asm);
                self.process_harnesses(tcx)
            });
        }

        self.prepare_codegen()
    }
}

#[cfg(test)]
mod tests {
    use super::{HarnessInfo, Stubs};
    use crate::kani_compiler::{group_by_stubs, HarnessId};
    use kani_metadata::{HarnessAttributes, HarnessMetadata};
    use rustc_data_structures::fingerprint::Fingerprint;
    use rustc_hir::definitions::DefPathHash;
    use std::collections::HashMap;

    fn mock_next_id() -> HarnessId {
        static mut COUNTER: u64 = 0;
        unsafe { COUNTER += 1 };
        let id = unsafe { COUNTER };
        DefPathHash(Fingerprint::new(id, 0))
    }

    fn mock_metadata() -> HarnessMetadata {
        HarnessMetadata {
            pretty_name: String::from("dummy"),
            mangled_name: String::from("dummy"),
            crate_name: String::from("dummy"),
            original_file: String::from("dummy"),
            original_start_line: 10,
            original_end_line: 20,
            goto_file: None,
            attributes: HarnessAttributes::default(),
        }
    }

    fn mock_info_with_stubs(stub_map: Stubs) -> HarnessInfo {
        HarnessInfo { metadata: mock_metadata(), stub_map }
    }

    #[test]
    fn test_group_by_stubs_works() {
        // Set up the inputs
        let harness_1 = mock_next_id();
        let harness_2 = mock_next_id();
        let harness_3 = mock_next_id();
        let harnesses = vec![harness_1, harness_2, harness_3];

        let stub_1 = (mock_next_id(), mock_next_id());
        let stub_2 = (mock_next_id(), mock_next_id());
        let stub_3 = (mock_next_id(), mock_next_id());
        let stub_4 = (stub_3.0, mock_next_id());

        let set_1 = Stubs::from([stub_1, stub_2, stub_3]);
        let set_2 = Stubs::from([stub_1, stub_2, stub_4]);
        let set_3 = Stubs::from([stub_1, stub_3, stub_2]);
        assert_eq!(set_1, set_3);
        assert_ne!(set_1, set_2);

        let harnesses_info = HashMap::from([
            (harness_1, mock_info_with_stubs(set_1)),
            (harness_2, mock_info_with_stubs(set_2)),
            (harness_3, mock_info_with_stubs(set_3)),
        ]);
        assert_eq!(harnesses_info.len(), 3);

        // Run the function under test.
        let grouped = group_by_stubs(harnesses, &harnesses_info);

        // Verify output.
        assert_eq!(grouped.len(), 2);
        assert!(
            grouped.contains(&vec![harness_1, harness_3])
                || grouped.contains(&vec![harness_3, harness_1])
        );
        assert!(grouped.contains(&vec![harness_2]));
    }
}
