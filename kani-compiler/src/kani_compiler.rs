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

use crate::args::{Arguments, ReachabilityType};
#[cfg(feature = "cprover")]
use crate::codegen_cprover_gotoc::{ContractInfoChannel, GotocCodegenBackend};
use crate::kani_middle::attributes::is_proof_harness;
use crate::kani_middle::check_crate_items;
use crate::kani_middle::metadata::gen_proof_metadata;
use crate::kani_middle::reachability::filter_crate_items;
use crate::kani_middle::stubbing::{self, harness_stub_map};
use crate::kani_queries::QueryDb;
use crate::session::init_session;
use cbmc::{InternString, InternedString};
use clap::Parser;
use kani_metadata::{ArtifactType, AssignsContract, HarnessMetadata, KaniMetadata};
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::definitions::DefPathHash;
use rustc_interface::Config;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::{ErrorOutputType, OutputType};
use rustc_smir::rustc_internal;
use rustc_span::ErrorGuaranteed;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::BufWriter;
use std::mem;
use std::path::{Path, PathBuf};
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
fn backend(
    queries: Arc<Mutex<QueryDb>>,
    contract_channel: ContractInfoChannel,
) -> Box<dyn CodegenBackend> {
    Box::new(GotocCodegenBackend::new(queries, contract_channel))
}

/// Fallback backend. It will trigger an error if no backend has been enabled.
#[cfg(not(feature = "cprover"))]
fn backend(queries: Arc<Mutex<QueryDb>>) -> Box<CodegenBackend> {
    compile_error!("No backend is available. Only supported value today is `cprover`");
}

/// A stable (across compilation sessions) identifier for the harness function.
type HarnessId = InternedString;

/// A set of stubs.
type Stubs = BTreeMap<DefPathHash, DefPathHash>;

#[derive(Clone, Debug)]
struct HarnessInfo {
    pub metadata: HarnessMetadata,
    pub stub_map: Stubs,
}

/// Store some relevant information about the crate compilation.
#[derive(Clone, Debug)]
struct CrateInfo {
    /// The name of the crate being compiled.
    pub name: String,
    /// The metadata output path that shall be generated as part of the crate compilation.
    pub output_path: PathBuf,
}

/// Represents the current compilation stage.
///
/// The Kani compiler may run the Rust compiler multiple times since stubbing has to be applied
/// to the entire Rust compiler session.
///
/// - We always start in the [CompilationStage::Init].
/// - After [CompilationStage::Init] we transition to either
///   - [CompilationStage::CodegenNoStubs] on a regular crate compilation, this will follow Init.
///   - [CompilationStage::CompilationSkipped], running the compiler to gather information, such as
///     `--version` will skip code generation completely, and there is no work to be done.
/// - After the [CompilationStage::CodegenNoStubs], we transition to either
///   - [CompilationStage::CodegenWithStubs] when there is at least one harness with stubs.
///   - [CompilationStage::Done] where there is no harness left to process.
/// - The [CompilationStage::CodegenWithStubs] can last multiple Rust compiler runs. Once there is
///   no harness left, we move to [CompilationStage::Done].
/// - The final stages are either [CompilationStage::Done] or [CompilationStage::CompilationSkipped].
///    - [CompilationStage::Done] represents the final state of the compiler after a successful
///      compilation. The crate metadata is stored here (even if no codegen was actually performed).
///    - [CompilationStage::CompilationSkipped] no compilation was actually performed.
///      No work needs to be done.
/// - Note: In a scenario where the compilation fails, the compiler will exit immediately,
///  independent on the stage. Any artifact produced shouldn't be used.
/// I.e.:
/// ```dot
/// graph CompilationStage {
///   Init -> {CodegenNoStubs, CompilationSkipped}
///   CodegenNoStubs -> {CodegenStubs, Done}
///   // Loop up to N harnesses times.
///   CodegenStubs -> {CodegenStubs, Done}
///   CompilationSkipped
///   Done
/// }
/// ```
#[allow(dead_code)]
#[derive(Debug)]
enum CompilationStage {
    /// Initial state that the compiler is always instantiated with.
    /// In this stage, we initialize the Query and collect all harnesses.
    Init,
    /// State where the compiler ran but didn't actually compile anything (e.g.: --version).
    CompilationSkipped,
    /// Stage where the compiler will perform codegen of all harnesses that don't use stub.
    CodegenNoStubs {
        target_harnesses: Vec<HarnessId>,
        next_harnesses: Vec<Vec<HarnessId>>,
        all_harnesses: HashMap<HarnessId, HarnessInfo>,
        crate_info: CrateInfo,
    },
    /// Stage where the compiler will codegen harnesses that use stub, one group at a time.
    /// The harnesses at this stage are grouped according to the stubs they are using. For now,
    /// we ensure they have the exact same set of stubs.
    CodegenWithStubs {
        target_harnesses: Vec<HarnessId>,
        next_harnesses: Vec<Vec<HarnessId>>,
        all_harnesses: HashMap<HarnessId, HarnessInfo>,
        crate_info: CrateInfo,
    },
    Done {
        metadata: Option<(KaniMetadata, CrateInfo)>,
    },
}

impl CompilationStage {
    pub fn is_init(&self) -> bool {
        matches!(self, CompilationStage::Init)
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
        loop {
            debug!(next=?self.stage, "run");
            match &self.stage {
                CompilationStage::Init => {
                    assert!(self.run_compilation_session(&orig_args)?.is_empty());
                }
                CompilationStage::CodegenNoStubs { .. } => {
                    unreachable!("This stage should always run in the same session as Init");
                }
                CompilationStage::CodegenWithStubs { target_harnesses, all_harnesses, .. } => {
                    assert!(!target_harnesses.is_empty(), "expected at least one target harness");
                    let target_harness_name = &target_harnesses[0];
                    let target_harness = &all_harnesses[target_harness_name];
                    let extra_arg = stubbing::mk_rustc_arg(&target_harness.stub_map);
                    let mut args = orig_args.clone();
                    args.push(extra_arg);
                    let contract_spec = self.run_compilation_session(&args)?;
                    let CompilationStage::CodegenWithStubs { all_harnesses, .. } = &mut self.stage
                    else {
                        unreachable!()
                    };
                    for (target, spec) in contract_spec {
                        let target_harness = all_harnesses.get_mut(&target).unwrap();
                        target_harness.metadata.contract = spec.into();
                    }
                }
                CompilationStage::Done { metadata: Some((kani_metadata, crate_info)) } => {
                    // Only store metadata for harnesses for now.
                    // TODO: This should only skip None.
                    // https://github.com/model-checking/kani/issues/2493
                    if self.queries.lock().unwrap().args().reachability_analysis
                        == ReachabilityType::Harnesses
                    {
                        // Store metadata file.
                        // We delay storing the metadata so we can include information collected
                        // during codegen.
                        self.store_metadata(&kani_metadata, &crate_info.output_path);
                    }
                    return Ok(());
                }
                CompilationStage::Done { metadata: None }
                | CompilationStage::CompilationSkipped => {
                    return Ok(());
                }
            };

            self.next_stage();
        }
    }

    /// Set up the next compilation stage after a `rustc` run.
    fn next_stage(&mut self) {
        self.stage = match &mut self.stage {
            CompilationStage::Init => {
                // This may occur when user passes arguments like --version or --help.
                CompilationStage::Done { metadata: None }
            }
            CompilationStage::CodegenNoStubs {
                next_harnesses, all_harnesses, crate_info, ..
            }
            | CompilationStage::CodegenWithStubs {
                next_harnesses,
                all_harnesses,
                crate_info,
                ..
            } => {
                if let Some(target_harnesses) = next_harnesses.pop() {
                    assert!(!target_harnesses.is_empty(), "expected at least one target harness");
                    CompilationStage::CodegenWithStubs {
                        target_harnesses,
                        next_harnesses: mem::take(next_harnesses),
                        all_harnesses: mem::take(all_harnesses),
                        crate_info: crate_info.clone(),
                    }
                } else {
                    CompilationStage::Done {
                        metadata: Some((
                            generate_metadata(&crate_info, &all_harnesses),
                            crate_info.clone(),
                        )),
                    }
                }
            }
            CompilationStage::Done { .. } | CompilationStage::CompilationSkipped => {
                unreachable!()
            }
        };
    }

    /// Run the Rust compiler with the given arguments and pass `&mut self` to handle callbacks.
    fn run_compilation_session(
        &mut self,
        args: &[String],
    ) -> Result<Vec<(InternedString, AssignsContract)>, ErrorGuaranteed> {
        debug!(?args, "run_compilation_session");
        let queries = self.queries.clone();
        let mut compiler = RunCompiler::new(args, self);
        let (send, receive) = std::sync::mpsc::channel();
        compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| backend(queries, send))));
        compiler.run()?;
        Ok(receive.iter().collect())
    }

    /// Gather and process all harnesses from this crate that shall be compiled.
    fn process_harnesses(&self, tcx: TyCtxt) -> CompilationStage {
        let crate_info = CrateInfo {
            name: tcx.crate_name(LOCAL_CRATE).as_str().into(),
            output_path: metadata_output_path(tcx),
        };
        if self.queries.lock().unwrap().args().reachability_analysis == ReachabilityType::Harnesses
        {
            let base_filename = tcx.output_filenames(()).output_path(OutputType::Object);
            let harnesses = filter_crate_items(tcx, |_, instance| is_proof_harness(tcx, instance));
            let all_harnesses = harnesses
                .into_iter()
                .map(|harness| {
                    let def_path = harness.mangled_name().intern();
                    let metadata = gen_proof_metadata(tcx, harness, &base_filename);
                    let stub_map = harness_stub_map(tcx, harness, &metadata);
                    (def_path, HarnessInfo { metadata, stub_map })
                })
                .collect::<HashMap<_, _>>();

            let (no_stubs, with_stubs): (Vec<_>, Vec<_>) =
                if self.queries.lock().unwrap().args().stubbing_enabled {
                    // Partition harnesses that don't have stub with the ones with stub.
                    all_harnesses
                        .keys()
                        .cloned()
                        .partition(|harness| all_harnesses[harness].stub_map.is_empty())
                } else {
                    // Generate code without stubs.
                    (all_harnesses.keys().cloned().collect(), vec![])
                };

            // Even if no_stubs is empty we still need to store rustc metadata.
            CompilationStage::CodegenNoStubs {
                target_harnesses: no_stubs,
                next_harnesses: group_by_stubs(with_stubs, &all_harnesses),
                all_harnesses,
                crate_info,
            }
        } else {
            // Leave other reachability type handling as is for now.
            CompilationStage::CodegenNoStubs {
                target_harnesses: vec![],
                next_harnesses: vec![],
                all_harnesses: HashMap::default(),
                crate_info,
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
            CompilationStage::Init
            | CompilationStage::Done { .. }
            | CompilationStage::CompilationSkipped => unreachable!(),
        }
    }

    /// Write the metadata to a file
    fn store_metadata(&self, metadata: &KaniMetadata, filename: &Path) {
        debug!(?filename, "write_metadata");
        let out_file = File::create(filename).unwrap();
        let writer = BufWriter::new(out_file);
        if self.queries.lock().unwrap().args().output_pretty_json {
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
            let args = Arguments::parse_from(args);
            init_session(&args, matches!(config.opts.error_format, ErrorOutputType::Json { .. }));
            // Configure queries.
            let queries = &mut (*self.queries.lock().unwrap());

            queries.set_args(args);

            debug!(?queries, "config end");
        }
    }

    /// During the initialization state, we collect the crate harnesses and prepare for codegen.
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        rustc_queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        if self.stage.is_init() {
            self.stage = rustc_queries.global_ctxt().unwrap().enter(|tcx| {
                rustc_internal::run(tcx, || {
                    check_crate_items(tcx, self.queries.lock().unwrap().args().ignore_global_asm);
                    self.process_harnesses(tcx)
                })
                .unwrap()
            })
        }

        self.prepare_codegen()
    }
}

/// Generate [KaniMetadata] for the target crate.
fn generate_metadata(
    crate_info: &CrateInfo,
    all_harnesses: &HashMap<HarnessId, HarnessInfo>,
) -> KaniMetadata {
    let (proof_harnesses, test_harnesses) = all_harnesses
        .values()
        .map(|info| &info.metadata)
        .cloned()
        .partition(|md| md.attributes.proof);
    KaniMetadata {
        crate_name: crate_info.name.clone(),
        proof_harnesses,
        unsupported_features: vec![],
        test_harnesses,
    }
}

/// Extract the filename for the metadata file.
fn metadata_output_path(tcx: TyCtxt) -> PathBuf {
    let mut filename = tcx.output_filenames(()).output_path(OutputType::Object);
    filename.set_extension(ArtifactType::Metadata);
    filename
}

#[cfg(test)]
mod tests {
    use super::*;
    use kani_metadata::{HarnessAttributes, HarnessMetadata};
    use rustc_data_structures::fingerprint::Fingerprint;
    use rustc_hir::definitions::DefPathHash;
    use std::collections::HashMap;

    fn mock_next_harness_id() -> HarnessId {
        static mut COUNTER: u64 = 0;
        unsafe { COUNTER += 1 };
        let id = unsafe { COUNTER };
        format!("mod::harness-{id}").intern()
    }

    fn mock_next_stub_id() -> DefPathHash {
        static mut COUNTER: u64 = 0;
        unsafe { COUNTER += 1 };
        let id = unsafe { COUNTER };
        DefPathHash(Fingerprint::new(id, 0))
    }

    fn mock_metadata(name: String, krate: String) -> HarnessMetadata {
        HarnessMetadata {
            pretty_name: name.clone(),
            mangled_name: name.clone(),
            original_file: format!("{}.rs", krate),
            crate_name: krate,
            original_start_line: 10,
            original_end_line: 20,
            goto_file: None,
            attributes: HarnessAttributes::default(),
            contract: Default::default(),
        }
    }

    fn mock_info_with_stubs(stub_map: Stubs) -> HarnessInfo {
        HarnessInfo { metadata: mock_metadata("dummy".to_string(), "crate".to_string()), stub_map }
    }

    #[test]
    fn test_group_by_stubs_works() {
        // Set up the inputs
        let harness_1 = mock_next_harness_id();
        let harness_2 = mock_next_harness_id();
        let harness_3 = mock_next_harness_id();
        let harnesses = vec![harness_1, harness_2, harness_3];

        let stub_1 = (mock_next_stub_id(), mock_next_stub_id());
        let stub_2 = (mock_next_stub_id(), mock_next_stub_id());
        let stub_3 = (mock_next_stub_id(), mock_next_stub_id());
        let stub_4 = (stub_3.0, mock_next_stub_id());

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

    #[test]
    fn test_generate_metadata() {
        // Mock inputs.
        let name = "my_crate".to_string();
        let crate_info = CrateInfo { name: name.clone(), output_path: PathBuf::default() };

        let mut info = mock_info_with_stubs(Stubs::default());
        info.metadata.attributes.proof = true;
        let id = mock_next_harness_id();
        let all_harnesses = HashMap::from([(id, info.clone())]);

        // Call generate metadata.
        let metadata = generate_metadata(&crate_info, &all_harnesses);

        // Check output.
        assert_eq!(metadata.crate_name, name);
        assert_eq!(metadata.proof_harnesses.len(), 1);
        assert_eq!(*metadata.proof_harnesses.first().unwrap(), info.metadata);
    }

    #[test]
    fn test_generate_empty_metadata() {
        // Mock inputs.
        let name = "my_crate".to_string();
        let crate_info = CrateInfo { name: name.clone(), output_path: PathBuf::default() };
        let all_harnesses = HashMap::new();

        // Call generate metadata.
        let metadata = generate_metadata(&crate_info, &all_harnesses);

        // Check output.
        assert_eq!(metadata.crate_name, name);
        assert_eq!(metadata.proof_harnesses.len(), 0);
    }

    #[test]
    fn test_generate_metadata_with_multiple_harness() {
        // Mock inputs.
        let krate = "my_crate".to_string();
        let crate_info = CrateInfo { name: krate.clone(), output_path: PathBuf::default() };

        let harnesses = ["h1", "h2", "h3"];
        let infos = harnesses.map(|harness| {
            let mut metadata = mock_metadata(harness.to_string(), krate.clone());
            metadata.attributes.proof = true;
            (mock_next_harness_id(), HarnessInfo { stub_map: Stubs::default(), metadata })
        });
        let all_harnesses = HashMap::from(infos.clone());

        // Call generate metadata.
        let metadata = generate_metadata(&crate_info, &all_harnesses);

        // Check output.
        assert_eq!(metadata.crate_name, krate);
        assert_eq!(metadata.proof_harnesses.len(), infos.len());
        assert!(
            metadata
                .proof_harnesses
                .iter()
                .all(|harness| harnesses.contains(&&*harness.pretty_name))
        );
    }
}
