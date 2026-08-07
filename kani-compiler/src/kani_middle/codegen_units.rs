// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module is responsible for extracting grouping harnesses that can be processed together
//! by codegen.
//!
//! Today, only stub / contracts can affect the harness codegen. Thus, we group the harnesses
//! according to their stub configuration.

use crate::args::{Arguments, ReachabilityType};
use crate::kani_middle::attributes::{KaniAttributes, is_proof_harness};
use crate::kani_middle::kani_functions::{KaniIntrinsic, KaniModel};
use crate::kani_middle::metadata::{
    gen_automatic_proof_metadata, gen_contracts_metadata, gen_proof_metadata,
};
use crate::kani_middle::reachability::filter_crate_items;
use crate::kani_middle::stubbing::{check_compatibility, harness_stub_map};
use crate::kani_middle::{can_derive_arbitrary, implements_arbitrary};
use crate::kani_queries::QueryDb;
use kani_metadata::{
    ArtifactType, AssignsContract, AutoHarnessMetadata, AutoHarnessSkipReason, HarnessMetadata,
    KaniMetadata, find_proof_harnesses,
};
use regex::RegexSet;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;
use rustc_infer::infer::TyCtxtInferExt;
use rustc_middle::ty::{self, TyCtxt, TypingMode};
use rustc_public::mir::mono::Instance;
use rustc_public::rustc_internal;
use rustc_public::ty::{
    FloatTy, FnDef, GenericArgKind, GenericArgs, IntTy, Region, RegionKind, RigidTy, Ty, TyConst,
    TyKind, UintTy,
};
use rustc_public::{CrateDef, CrateItem};
use rustc_public_bridge::IndexedVal;
use rustc_session::config::OutputType;
use rustc_trait_selection::traits::{Obligation, ObligationCause, ObligationCtxt};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::debug;

/// An identifier for the harness function.
pub type Harness = Instance;

/// A set of stubs.
pub type Stubs = HashMap<FnDef, FnDef>;

static AUTOHARNESS_MD: OnceLock<AutoHarnessMetadata> = OnceLock::new();

/// Store some relevant information about the crate compilation.
#[derive(Clone, Debug)]
struct CrateInfo {
    /// The name of the crate being compiled.
    pub name: String,
}

/// We group the harnesses that have the same stubs.
pub struct CodegenUnits {
    crate_info: CrateInfo,
    harness_info: HashMap<Harness, HarnessMetadata>,
    units: Vec<CodegenUnit>,
}

#[derive(Clone, Default, Debug)]
pub struct CodegenUnit {
    pub harnesses: Vec<Harness>,
    pub stubs: Stubs,
}

impl CodegenUnits {
    pub fn new(queries: &QueryDb, tcx: TyCtxt) -> Self {
        let crate_info = CrateInfo { name: rustc_public::local_crate().name.as_str().into() };
        let base_filepath = tcx.output_filenames(()).path(OutputType::Object);
        let base_filename = base_filepath.as_path();
        let args = queries.args();
        match args.reachability_analysis {
            ReachabilityType::Harnesses => {
                let all_harnesses = determine_targets(
                    get_all_manual_harnesses(tcx, base_filename),
                    &args.harnesses,
                    args.exact,
                );
                // Even if no_stubs is empty we still need to store rustc metadata.
                let units = group_by_stubs(tcx, &all_harnesses);
                validate_units(tcx, &units);
                debug!(?units, "CodegenUnits::new");
                CodegenUnits { units, harness_info: all_harnesses, crate_info }
            }
            ReachabilityType::AllFns => {
                let mut all_harnesses = determine_targets(
                    get_all_manual_harnesses(tcx, base_filename),
                    &args.harnesses,
                    args.exact,
                );
                let mut units = group_by_stubs(tcx, &all_harnesses);
                validate_units(tcx, &units);

                let kani_fns = queries.kani_functions();
                let kani_harness_intrinsic =
                    kani_fns.get(&KaniIntrinsic::AutomaticHarness.into()).unwrap();

                let (chosen, skipped) = automatic_harness_partition(
                    tcx,
                    args,
                    &crate_info.name,
                    *kani_fns.get(&KaniModel::Any.into()).unwrap(),
                    &NondetFnModels {
                        fn0: kani_fns.get(&KaniModel::NondetFn0.into()).copied(),
                        fn1: kani_fns.get(&KaniModel::NondetFn1.into()).copied(),
                        fn1_ref: kani_fns.get(&KaniModel::NondetFn1Ref.into()).copied(),
                        fn2: kani_fns.get(&KaniModel::NondetFn2.into()).copied(),
                        fn2_ref_ref: kani_fns.get(&KaniModel::NondetFn2RefRef.into()).copied(),
                        fn2_ref_val: kani_fns.get(&KaniModel::NondetFn2RefVal.into()).copied(),
                        fn2_val_ref: kani_fns.get(&KaniModel::NondetFn2ValRef.into()).copied(),
                        fn3: kani_fns.get(&KaniModel::NondetFn3.into()).copied(),
                    },
                );
                AUTOHARNESS_MD
                    .set(AutoHarnessMetadata {
                        chosen: chosen
                            .iter()
                            .map(|func| crate::kani_middle::strip_local_crate_prefix(func.name()))
                            .collect::<BTreeSet<_>>(),
                        skipped,
                    })
                    .expect("Initializing the autoharness metadata failed");

                let automatic_harnesses = get_all_automatic_harnesses(
                    tcx,
                    chosen,
                    *kani_harness_intrinsic,
                    base_filename,
                );
                // We generate one contract harness per function under contract, so each harness is in its own unit,
                // and these harnesses have no stubs.
                units.extend(
                    automatic_harnesses
                        .keys()
                        .map(|harness| CodegenUnit {
                            harnesses: vec![*harness],
                            stubs: HashMap::default(),
                        })
                        .collect::<Vec<_>>(),
                );
                all_harnesses.extend(automatic_harnesses.clone());

                // No need to validate the units again because validation only checks stubs, and we haven't added any stubs.
                debug!(?units, "CodegenUnits::new");
                CodegenUnits { units, harness_info: all_harnesses, crate_info }
            }
            _ => {
                // Leave other reachability type handling as is for now.
                CodegenUnits { units: vec![], harness_info: HashMap::default(), crate_info }
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &CodegenUnit> {
        self.units.iter()
    }

    pub fn is_automatic_harness(&self, harness: &Harness) -> bool {
        self.harness_info.get(harness).is_some_and(|md| md.is_automatically_generated)
    }

    /// We store which instance of modifies was generated.
    pub fn store_modifies(&mut self, harness_modifies: &[(Harness, AssignsContract)]) {
        for (harness, modifies) in harness_modifies {
            self.harness_info.get_mut(harness).unwrap().contract = Some(modifies.clone());
        }
    }

    /// We flag that the harness contains usage of loop contracts.
    pub fn store_loop_contracts(&mut self, harnesses: &[Harness]) {
        for harness in harnesses {
            let metadata = self.harness_info.get_mut(harness).unwrap();
            metadata.has_loop_contracts = true;
        }
    }

    /// Write compilation metadata into a file.
    pub fn write_metadata(&self, queries: &QueryDb, tcx: TyCtxt) {
        let metadata = self.generate_metadata(tcx);
        let outpath = metadata_output_path(tcx);
        store_metadata(queries, &metadata, &outpath);
    }

    pub fn harness_model_path(&self, harness: Harness) -> Option<&PathBuf> {
        self.harness_info[&harness].goto_file.as_ref()
    }

    /// Generate [KaniMetadata] for the target crate.
    fn generate_metadata(&self, tcx: TyCtxt) -> KaniMetadata {
        let (proof_harnesses, test_harnesses) =
            self.harness_info.values().cloned().partition(|md| md.attributes.is_proof_harness());
        KaniMetadata {
            crate_name: self.crate_info.name.clone(),
            proof_harnesses,
            unsupported_features: vec![],
            test_harnesses,
            contracted_functions: gen_contracts_metadata(tcx, &self.harness_info),
            autoharness_md: AUTOHARNESS_MD.get().cloned(),
        }
    }
}

fn stub_def(tcx: TyCtxt, def_id: DefId) -> FnDef {
    let ty_internal = tcx.type_of(def_id).instantiate_identity();
    let ty = rustc_internal::stable(ty_internal);
    if let TyKind::RigidTy(RigidTy::FnDef(def, _)) = ty.kind() {
        def
    } else {
        unreachable!("Expected stub function for `{:?}`, but found: {ty}", tcx.def_path(def_id))
    }
}

/// Group the harnesses by their stubs and contract usage.
fn group_by_stubs(
    tcx: TyCtxt,
    all_harnesses: &HashMap<Harness, HarnessMetadata>,
) -> Vec<CodegenUnit> {
    let mut per_stubs: HashMap<_, CodegenUnit> = HashMap::default();
    for (harness, metadata) in all_harnesses {
        let stub_ids = harness_stub_map(tcx, *harness, metadata);
        let contracts = extract_contracts(tcx, *harness);
        let stub_map = stub_ids
            .iter()
            .map(|(k, v)| (tcx.def_path_hash(*k), tcx.def_path_hash(*v)))
            .collect::<BTreeMap<_, _>>();
        let key = (contracts, stub_map);
        if let Some(unit) = per_stubs.get_mut(&key) {
            unit.harnesses.push(*harness);
        } else {
            let stubs = stub_ids
                .iter()
                .map(|(from, to)| (stub_def(tcx, *from), stub_def(tcx, *to)))
                .collect::<HashMap<_, _>>();
            let stubs = apply_transitivity(tcx, *harness, stubs);
            per_stubs.insert(key, CodegenUnit { stubs, harnesses: vec![*harness] });
        }
    }
    per_stubs.into_values().collect()
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Hash)]
enum ContractUsage {
    Stub(usize),
    Check(usize),
}

/// Extract the contract related usages.
///
/// Note that any error interpreting the result is emitted, but we delay aborting, so we emit as
/// many errors as possible.
fn extract_contracts(tcx: TyCtxt, harness: Harness) -> BTreeSet<ContractUsage> {
    let def = harness.def;
    let mut result = BTreeSet::new();
    let attributes = KaniAttributes::for_def_id(tcx, def.def_id());
    if let Some(target) = attributes.interpret_for_contract_attribute() {
        result.insert(ContractUsage::Check(target.def_id().to_index()));
    }
    for stub in attributes.interpret_stub_verified_attribute() {
        result.insert(ContractUsage::Stub(stub.def_id().to_index()));
    }

    result
}

/// Extract the filename for the metadata file.
fn metadata_output_path(tcx: TyCtxt) -> PathBuf {
    let filepath = tcx.output_filenames(()).path(OutputType::Object);
    let filename = filepath.as_path();
    filename.with_extension(ArtifactType::Metadata).to_path_buf()
}

/// Write the metadata to a file
fn store_metadata(queries: &QueryDb, metadata: &KaniMetadata, filename: &Path) {
    debug!(?filename, "store_metadata");
    let out_file = File::create(filename).unwrap();
    let writer = BufWriter::new(out_file);
    if queries.args().output_pretty_json {
        serde_json::to_writer_pretty(writer, &metadata).unwrap();
    } else {
        serde_json::to_writer(writer, &metadata).unwrap();
    }
}

/// Validate the unit configuration.
fn validate_units(tcx: TyCtxt, units: &[CodegenUnit]) {
    for unit in units {
        for (from, to) in &unit.stubs {
            // We use harness span since we don't keep the attribute span.
            let Err(msg) = check_compatibility(tcx, *from, *to) else { continue };
            let span = unit.harnesses.first().unwrap().def.span();
            tcx.dcx().span_err(rustc_internal::internal(tcx, span), msg);
        }
    }
    tcx.dcx().abort_if_errors();
}

/// Apply stub transitivity operations.
///
/// If `fn1` is stubbed by `fn2`, and `fn2` is stubbed by `fn3`, `f1` is in fact stubbed by `fn3`.
fn apply_transitivity(tcx: TyCtxt, harness: Harness, stubs: Stubs) -> Stubs {
    let mut new_stubs = Stubs::with_capacity(stubs.len());
    for (orig, new) in stubs.iter() {
        let mut new_fn = *new;
        let mut visited = HashSet::new();
        while let Some(stub) = stubs.get(&new_fn) {
            if !visited.insert(stub) {
                // Visiting the same stub, i.e. found cycle.
                let span = harness.def.span();
                tcx.dcx().span_err(
                    rustc_internal::internal(tcx, span),
                    format!(
                        "Cannot stub `{}`. Stub configuration for harness `{}` has a cycle",
                        orig.name(),
                        crate::kani_middle::strip_local_crate_prefix(harness.def.name()),
                    ),
                );
                break;
            }
            new_fn = *stub;
        }
        new_stubs.insert(*orig, new_fn);
    }
    new_stubs
}

/// Fetch all manual harnesses (i.e., functions provided by the user) and generate their metadata
fn get_all_manual_harnesses(
    tcx: TyCtxt,
    base_filename: &Path,
) -> HashMap<Harness, HarnessMetadata> {
    let harnesses = filter_crate_items(tcx, |_, instance| is_proof_harness(tcx, instance));
    harnesses
        .into_iter()
        .map(|harness| {
            let metadata = gen_proof_metadata(tcx, harness, base_filename);
            (harness, metadata)
        })
        .collect::<HashMap<_, _>>()
}

/// Filter which harnesses to codegen based on user filters. Shares use of `find_proof_harnesses` with the `determine_targets` function
/// in `kani-driver/src/metadata.rs` to ensure the filter is consistent and thus codegen is always done for the subset of harnesses we want
/// to analyze.
fn determine_targets(
    all_harnesses: HashMap<Harness, HarnessMetadata>,
    harness_filters: &[String],
    exact_filter: bool,
) -> HashMap<Harness, HarnessMetadata> {
    if harness_filters.is_empty() {
        return all_harnesses;
    }

    // If there are filters, only keep around harnesses that satisfy them.
    let mut new_harnesses = all_harnesses.clone();
    let valid_harnesses = find_proof_harnesses(
        &BTreeSet::from_iter(harness_filters.iter()),
        all_harnesses.values(),
        exact_filter,
    );

    new_harnesses.retain(|_, metadata| valid_harnesses.contains(&&*metadata));
    new_harnesses
}

/// For each function eligible for automatic verification,
/// generate a harness Instance for it, then generate its metadata.
/// Note that the body of each harness instance is still the dummy body of `kani_harness_intrinsic`;
/// the AutomaticHarnessPass will later transform the bodies of these instances to actually verify the function.
fn get_all_automatic_harnesses(
    tcx: TyCtxt,
    verifiable_fns: Vec<Instance>,
    kani_harness_intrinsic: FnDef,
    base_filename: &Path,
) -> HashMap<Harness, HarnessMetadata> {
    verifiable_fns
        .into_iter()
        .map(|fn_to_verify| {
            // Set the generic arguments of the harness to be the function it is verifying
            // so that later, in AutomaticHarnessPass, we can retrieve the function to verify
            // and generate the harness body accordingly.
            let harness = Instance::resolve(
                kani_harness_intrinsic,
                &GenericArgs(vec![GenericArgKind::Type(fn_to_verify.ty())]),
            )
            .unwrap();
            let metadata = gen_automatic_proof_metadata(
                tcx,
                base_filename,
                &fn_to_verify,
                harness.mangled_name(),
            );
            (harness, metadata)
        })
        .collect::<HashMap<_, _>>()
}

fn make_regex_set(patterns: Vec<String>) -> Option<RegexSet> {
    if patterns.is_empty() {
        None
    } else {
        Some(RegexSet::new(patterns).unwrap_or_else(|e| {
            panic!("Invalid regexes should have been caught during argument validation: {e}")
        }))
    }
}

/// A function is filtered out if 1) none of the include patterns match it or 2) one of the exclude patterns matches it.
fn autoharness_filtered_out(
    name: &str,
    included_set: &Option<RegexSet>,
    excluded_set: &Option<RegexSet>,
) -> bool {
    // A function is included if `--include-pattern` is not provided or if at least one of its regexes matches `name`
    let included = included_set.as_ref().is_none_or(|set| set.is_match(name));
    // A function is excluded if `--exclude-pattern` is provided and at least one of its regexes matches `name`
    let excluded = excluded_set.as_ref().is_some_and(|set| set.is_match(name));
    !included || excluded
}

/// The value used to instantiate `usize` const generic parameters of generic functions
/// (e.g. array lengths). As with the choice of type-parameter candidates, verifying a single
/// instantiation underapproximates the function's behaviors; the summary table shows the
/// chosen value as part of the instantiated name.
const AUTOHARNESS_CONST_GENERIC_VALUE: u64 = 2;

/// The candidate types for instantiating the type parameters of a generic function, in the order
/// in which we try them. We start with `i32` since that is Rust's default integer type, and
/// primitive types satisfy the most common trait bounds (`Copy`, `Clone`, `Ord`, `Hash`,
/// `Default`, `Debug`, etc.) as well as Kani's `Arbitrary`.
fn generic_instantiation_candidates() -> Vec<Ty> {
    vec![
        Ty::from_rigid_kind(RigidTy::Int(IntTy::I32)),
        Ty::from_rigid_kind(RigidTy::Uint(UintTy::U32)),
        Ty::from_rigid_kind(RigidTy::Uint(UintTy::Usize)),
        Ty::from_rigid_kind(RigidTy::Uint(UintTy::U8)),
        Ty::from_rigid_kind(RigidTy::Int(IntTy::I64)),
        Ty::from_rigid_kind(RigidTy::Uint(UintTy::U64)),
        Ty::from_rigid_kind(RigidTy::Float(FloatTy::F64)),
        Ty::from_rigid_kind(RigidTy::Float(FloatTy::F32)),
        Ty::from_rigid_kind(RigidTy::Bool),
        Ty::from_rigid_kind(RigidTy::Char),
    ]
}

/// Cap on trait-solver queries per function when searching for a satisfying instantiation,
/// so that functions with many type parameters do not blow up partitioning time.
const GENERIC_INSTANTIATION_ATTEMPT_LIMIT: usize = 256;

/// Cap on the number of trait-impl-derived candidate types collected per type parameter.
const IMPL_DERIVED_CANDIDATE_LIMIT: usize = 16;

/// For each type parameter of `def` (keyed by its index in the generic parameter list),
/// collect concrete types that implement the parameter's trait bounds, by enumerating the
/// non-blanket implementations of each trait the parameter is bound by. This finds candidates
/// for parameters bound by crate-local or third-party traits (e.g. num-traits' `Float`),
/// which no primitive candidate may satisfy.
/// Candidates are deduplicated, restricted to fully concrete types, and sorted for
/// determinism; each parameter's list is capped at [IMPL_DERIVED_CANDIDATE_LIMIT].
fn impl_derived_candidates(tcx: TyCtxt, def: FnDef) -> FxHashMap<usize, Vec<Ty>> {
    let def_id = rustc_internal::internal(tcx, def.def_id());
    let mut candidates: FxHashMap<usize, Vec<Ty>> = FxHashMap::default();
    for (predicate, _span) in tcx.predicates_of(def_id).predicates {
        let Some(trait_pred) = predicate.as_trait_clause() else { continue };
        let trait_pred = trait_pred.skip_binder();
        let ty::Param(param_ty) = trait_pred.self_ty().kind() else { continue };
        let slot = candidates.entry(param_ty.index as usize).or_default();
        for impls in tcx.trait_impls_of(trait_pred.def_id()).non_blanket_impls().values() {
            for &impl_def_id in impls {
                let self_ty = tcx.type_of(impl_def_id).instantiate_identity();
                // Only fully concrete self types can be substituted directly.
                if rustc_middle::ty::TypeVisitableExt::has_param(&self_ty) {
                    continue;
                }
                let stable_ty = rustc_internal::stable(self_ty);
                if !slot.contains(&stable_ty) {
                    slot.push(stable_ty);
                }
            }
        }
    }
    for slot in candidates.values_mut() {
        slot.sort_by_key(|ty| ty.to_string());
        slot.truncate(IMPL_DERIVED_CANDIDATE_LIMIT);
    }
    candidates
}

/// Check whether instantiating the generic parameters of `def` with `args` satisfies all of
/// `def`'s predicates (trait bounds and where clauses).
/// `args` must be fully monomorphic.
fn args_satisfy_predicates(tcx: TyCtxt, def: FnDef, args: &GenericArgs) -> bool {
    let infcx = tcx.infer_ctxt().build(TypingMode::PostAnalysis);
    let ocx = ObligationCtxt::new(&infcx);
    let param_env = ty::ParamEnv::empty();
    let cause = ObligationCause::dummy();

    let def_id = rustc_internal::internal(tcx, def.def_id());
    let args_internal = rustc_internal::internal(tcx, args);
    let predicates = tcx.predicates_of(def_id).instantiate(tcx, args_internal);
    for (predicate, _span) in predicates {
        ocx.register_obligation(Obligation::new(tcx, cause.clone(), param_env, predicate));
    }
    ocx.evaluate_obligations_error_on_ambiguity().is_empty()
}

/// Try to find a monomorphic instantiation of the generic function `fn_item` for which we can
/// generate an automatic harness. Substitute each type parameter with the first candidate from
/// `generic_instantiation_candidates` such that all of the function's trait bounds are satisfied
/// (using the same candidate for every type parameter), and erase lifetime parameters.
/// Return the reason (to be attached to [AutoHarnessSkipReason::GenericFn]) if no candidate
/// satisfies the bounds, or if the function has const generic parameters, which we do not
/// support instantiating yet.
/// For type parameters bound by `Fn`/`FnMut`/`FnOnce`, derive a candidate instantiation:
/// the *function item type* of the matching-arity `kani::arbitrary::nondet_fn<N>` model,
/// instantiated with the bound's argument and return types. Function items implement all
/// three `Fn` traits and are zero-sized; the models return a fresh nondeterministic value
/// per call, over-approximating every real closure's behavior.
///
/// Returns a map from parameter index (among the identity args) to candidate types.
/// Signature types that themselves mention generic parameters are only usable if those
/// parameters appear EARLIER in the parameter list (they are substituted with the current
/// choice by the caller); v1 keeps it simple and only admits fully concrete signatures.
/// The nondet closure-model FnDefs, keyed by input shape. By-value models fix their
/// input regions early-bound; the ref-taking variants carry late-bound regions so their
/// fn items satisfy HRTB bounds like `for<'a> Fn(&'a T)`.
#[derive(Clone, Copy, Default)]
pub struct NondetFnModels {
    pub fn0: Option<FnDef>,
    pub fn1: Option<FnDef>,
    pub fn1_ref: Option<FnDef>,
    pub fn2: Option<FnDef>,
    pub fn2_ref_ref: Option<FnDef>,
    pub fn2_ref_val: Option<FnDef>,
    pub fn2_val_ref: Option<FnDef>,
    pub fn3: Option<FnDef>,
}

/// Select the nondet model matching the (erased-region) input types' by-ref/by-value
/// shape, returning the model and its type arguments (references peeled: the model's own
/// signature reintroduces them with late-bound regions). Arity-3 ref shapes and deeper
/// are not modeled (1.5% corpus tail).
fn select_nondet_model<'tcx>(
    models: &NondetFnModels,
    input_tys: &[rustc_middle::ty::Ty<'tcx>],
) -> Option<(FnDef, Vec<rustc_middle::ty::Ty<'tcx>>)> {
    let peel = |t: rustc_middle::ty::Ty<'tcx>| match t.kind() {
        rustc_middle::ty::TyKind::Ref(_, inner, rustc_middle::ty::Mutability::Not) => Some(*inner),
        _ => None,
    };
    let shape: Vec<Option<rustc_middle::ty::Ty>> = input_tys.iter().map(|t| peel(*t)).collect();
    match shape.as_slice() {
        [] => models.fn0.map(|m| (m, vec![])),
        [None] => models.fn1.map(|m| (m, vec![input_tys[0]])),
        [Some(t)] => models.fn1_ref.map(|m| (m, vec![*t])),
        [None, None] => models.fn2.map(|m| (m, input_tys.to_vec())),
        [Some(a), Some(b)] => models.fn2_ref_ref.map(|m| (m, vec![*a, *b])),
        [Some(a), None] => models.fn2_ref_val.map(|m| (m, vec![*a, input_tys[1]])),
        [None, Some(b)] => models.fn2_val_ref.map(|m| (m, vec![input_tys[0], *b])),
        [None, None, None] => models.fn3.map(|m| (m, input_tys.to_vec())),
        _ => None,
    }
}

/// An Fn-bound signature that references other generic parameters (e.g. `F: Fn(T) -> T`):
/// its concrete form depends on the instantiation chosen for those parameters, so the
/// candidate fn-item type is constructed per candidate choice
/// (c.f. [resolve_deferred_fn_slots]).
struct DeferredFnSpec<'tcx> {
    inputs: rustc_middle::ty::Ty<'tcx>,
    output: rustc_middle::ty::Ty<'tcx>,
}

fn fn_bound_candidates<'tcx>(
    tcx: TyCtxt<'tcx>,
    def: FnDef,
    nondet_fns: &NondetFnModels,
) -> (FxHashMap<usize, Vec<Ty>>, FxHashMap<usize, DeferredFnSpec<'tcx>>) {
    let def_id = rustc_internal::internal(tcx, def.def_id());
    let mut out: FxHashMap<usize, Vec<Ty>> = FxHashMap::default();
    let mut deferred: FxHashMap<usize, DeferredFnSpec<'tcx>> = FxHashMap::default();
    let fn_once = tcx.lang_items().fn_once_trait();
    let fn_mut = tcx.lang_items().fn_mut_trait();
    let fn_tr = tcx.lang_items().fn_trait();
    // Collect Fn-ish trait predicates keyed by the self param index, with tupled inputs.
    let mut sig_inputs: FxHashMap<usize, rustc_middle::ty::Ty> = FxHashMap::default();
    for (predicate, _span) in tcx.predicates_of(def_id).predicates {
        let Some(tp) = predicate.as_trait_clause() else { continue };
        // HRTB bounds (e.g. for<'a> FnOnce(&'a Self)) carry late-bound regions; erase them
        // rather than skipping the binder, which would leak escaping bound vars into the
        // trait solver (ICE: !self_ty.has_escaping_bound_vars()).
        let tp = tcx.instantiate_bound_regions_with_erased(tp);
        let tid = Some(tp.def_id());
        if tid != fn_once && tid != fn_mut && tid != fn_tr {
            continue;
        }
        let rustc_middle::ty::TyKind::Param(param_ty) = tp.self_ty().kind() else { continue };
        // Second generic arg of the Fn traits is the tupled inputs.
        let Some(inputs) = tp.trait_ref.args.get(1).and_then(|a| a.as_type()) else {
            continue;
        };
        sig_inputs.insert(param_ty.index as usize, inputs);
    }
    if sig_inputs.is_empty() {
        return (out, deferred);
    }
    // The return type comes from the FnOnce::Output projection bound.
    let mut sig_output: FxHashMap<usize, rustc_middle::ty::Ty> = FxHashMap::default();
    for (predicate, _span) in tcx.predicates_of(def_id).predicates {
        let Some(proj) = predicate.as_projection_clause() else { continue };
        let proj = tcx.instantiate_bound_regions_with_erased(proj);
        let rustc_middle::ty::TyKind::Param(param_ty) = proj.projection_term.self_ty().kind()
        else {
            continue;
        };
        if let Some(term_ty) = proj.term.as_type() {
            sig_output.insert(param_ty.index as usize, term_ty);
        }
    }
    for (idx, inputs) in sig_inputs {
        let rustc_middle::ty::TyKind::Tuple(input_tys) = inputs.kind() else { continue };
        let output = sig_output.get(&idx).copied().unwrap_or(tcx.types.unit);
        use rustc_middle::ty::TypeVisitableExt;
        if inputs.has_param() || output.has_param() {
            // Signature references other generic parameters: defer construction until a
            // candidate choice for those parameters is made.
            // SAFETY of the transmute-free 'static: predicates_of types live for the whole
            // compilation session ('tcx); we only use them within this query's lifetime.
            deferred.insert(idx, DeferredFnSpec { inputs, output });
            continue;
        }
        // nondet_fnN<A.., R>: generic args are the inputs followed by the return type.
        let input_vec: Vec<rustc_middle::ty::Ty> = input_tys.iter().collect();
        let Some((model, model_tys)) = select_nondet_model(nondet_fns, &input_vec) else {
            continue;
        };
        let mut args: Vec<GenericArgKind> =
            model_tys.iter().map(|t| GenericArgKind::Type(rustc_internal::stable(t))).collect();
        args.push(GenericArgKind::Type(rustc_internal::stable(output)));
        let args = GenericArgs(args);
        // Instance::resolve does not check trait bounds; the model requires R: Arbitrary
        // (its body calls kani::any::<R>()), so verify the model's own predicates or the
        // assert in harness generation fires (e.g. FnOnce() -> error::Error in syn).
        if !args_satisfy_predicates(tcx, model, &args) {
            continue;
        }
        let Ok(inst) = Instance::resolve(model, &args) else { continue };
        // The function item TYPE of the resolved instance.
        out.entry(idx).or_default().push(inst.ty());
    }
    (out, deferred)
}

/// Resolve deferred Fn-bound slots for a concrete candidate `choice`: substitute the
/// chosen types into the deferred signature, construct the matching-arity nondet_fn item
/// type, and overwrite the placeholder in `choice`. Returns false if any deferred slot
/// cannot be resolved for this choice (skip it).
#[allow(clippy::too_many_arguments)]
fn resolve_deferred_fn_slots<'tcx>(
    tcx: TyCtxt<'tcx>,
    identity_args: &GenericArgs,
    type_slots: &[usize],
    choice: &mut [Ty],
    deferred: &FxHashMap<usize, DeferredFnSpec<'tcx>>,
    nondet_fns: &NondetFnModels,
) -> bool {
    if deferred.is_empty() {
        return true;
    }
    // Build a full internal substitution from the current choice (placeholders included:
    // deferred slots hold unit, which is fine as long as no deferred signature references
    // another Fn-bound parameter).
    let mut next_type = 0usize;
    let stable_args = GenericArgs(
        identity_args
            .0
            .iter()
            .map(|arg| match arg {
                GenericArgKind::Type(_) => {
                    let t = choice[next_type];
                    next_type += 1;
                    GenericArgKind::Type(t)
                }
                GenericArgKind::Lifetime(_) => {
                    GenericArgKind::Lifetime(Region { kind: RegionKind::ReErased })
                }
                GenericArgKind::Const(_) => GenericArgKind::Const(
                    TyConst::try_from_target_usize(AUTOHARNESS_CONST_GENERIC_VALUE).unwrap(),
                ),
            })
            .collect(),
    );
    let args_internal = rustc_internal::internal(tcx, &stable_args);
    for (&idx, spec) in deferred {
        use rustc_middle::ty::TypeVisitableExt;
        let inputs =
            rustc_middle::ty::EarlyBinder::bind(spec.inputs).instantiate(tcx, args_internal);
        let output =
            rustc_middle::ty::EarlyBinder::bind(spec.output).instantiate(tcx, args_internal);
        if inputs.has_param() || output.has_param() {
            return false;
        }
        // The substitution may produce unnormalizable projections (e.g. <i32 as Tap>::Val
        // for a choice that does not satisfy the bound); normalize here and skip the
        // choice on failure, rather than letting Instance::resolve ICE on it.
        let typing_env = rustc_middle::ty::TypingEnv::fully_monomorphized();
        let Ok(inputs) = tcx.try_normalize_erasing_regions(typing_env, inputs) else {
            return false;
        };
        let Ok(output) = tcx.try_normalize_erasing_regions(typing_env, output) else {
            return false;
        };
        let rustc_middle::ty::TyKind::Tuple(input_tys) = inputs.kind() else { return false };
        let input_vec: Vec<rustc_middle::ty::Ty> = input_tys.iter().collect();
        let Some((model, model_tys)) = select_nondet_model(nondet_fns, &input_vec) else {
            return false;
        };
        let mut margs: Vec<GenericArgKind> =
            model_tys.iter().map(|t| GenericArgKind::Type(rustc_internal::stable(t))).collect();
        margs.push(GenericArgKind::Type(rustc_internal::stable(output)));
        let margs = GenericArgs(margs);
        // As in fn_bound_candidates: enforce the model's own R: Arbitrary bound.
        if !args_satisfy_predicates(tcx, model, &margs) {
            return false;
        }
        let Ok(inst) = Instance::resolve(model, &margs) else { return false };
        let Some(pos) = type_slots.iter().position(|&s| s == idx) else { return false };
        choice[pos] = inst.ty();
    }
    true
}

fn choose_generic_instantiation(
    tcx: TyCtxt,
    fn_item: CrateItem,
    nondet_fns: &NondetFnModels,
) -> Result<Instance, String> {
    let TyKind::RigidTy(RigidTy::FnDef(def, identity_args)) = fn_item.ty().kind() else {
        return Err("not a function definition".to_string());
    };

    // Const generic parameters of type usize (by far the most common case, e.g. array lengths)
    // are instantiated with AUTOHARNESS_CONST_GENERIC_VALUE; other const parameter types are
    // not supported yet. The check consults the internal generics (the public identity
    // arguments do not carry the parameter's type).
    let generics = tcx.generics_of(rustc_internal::internal(tcx, def.def_id()));
    if generics.own_params.iter().any(|param| {
        matches!(param.kind, rustc_middle::ty::GenericParamDefKind::Const { .. })
            && tcx.type_of(param.def_id).skip_binder() != tcx.types.usize
    }) {
        return Err("non-usize const generic parameters are not supported yet".to_string());
    }

    // Positions of the type parameters among the identity arguments, and the candidate list
    // for each: the shared primitive candidates, plus types derived from the parameter's own
    // trait bounds (concrete implementors of the traits it must satisfy).
    let impl_derived = impl_derived_candidates(tcx, def);
    let (fn_bound, deferred_fn) = fn_bound_candidates(tcx, def, nondet_fns);
    let type_slots: Vec<usize> = identity_args
        .0
        .iter()
        .enumerate()
        .filter_map(|(idx, arg)| matches!(arg, GenericArgKind::Type(_)).then_some(idx))
        .collect();
    let slot_candidates: Vec<Vec<Ty>> = type_slots
        .iter()
        .map(|&idx| {
            let mut cands = generic_instantiation_candidates();
            for ty in impl_derived.get(&idx).into_iter().flatten() {
                if !cands.contains(ty) {
                    cands.push(*ty);
                }
            }
            // Fn-bound parameters: try the nondeterministic function items FIRST — no
            // primitive can satisfy an Fn bound, so they would only waste solver queries.
            if let Some(fnc) = fn_bound.get(&idx) {
                let mut new = fnc.clone();
                new.extend(cands);
                cands = new;
            } // Deferred Fn-bound slots get a single placeholder (resolved per choice by
            // resolve_deferred_fn_slots); other candidates would be wasted solver queries.
            if deferred_fn.contains_key(&idx) {
                cands = vec![Ty::new_tuple(&[])];
            }
            cands
        })
        .collect();
    let n_impl_derived: usize = impl_derived.values().map(|v| v.len()).sum();

    // Build the argument list substituting `choice[i]` for the i-th type parameter.
    let build_args = |choice: &[Ty]| {
        let mut next_type = 0;
        GenericArgs(
            identity_args
                .0
                .iter()
                .map(|arg| match arg {
                    GenericArgKind::Type(_) => {
                        let ty = choice[next_type];
                        next_type += 1;
                        GenericArgKind::Type(ty)
                    }
                    GenericArgKind::Lifetime(_) => {
                        GenericArgKind::Lifetime(Region { kind: RegionKind::ReErased })
                    }
                    GenericArgKind::Const(_) => GenericArgKind::Const(
                        TyConst::try_from_target_usize(AUTOHARNESS_CONST_GENERIC_VALUE).unwrap(),
                    ),
                })
                .collect(),
        )
    };

    let attempts = std::cell::Cell::new(0usize);
    let try_choice = |choice: &[Ty]| -> Option<Instance> {
        attempts.set(attempts.get() + 1);
        let args = build_args(choice);
        if !args_satisfy_predicates(tcx, def, &args) {
            return None;
        }
        match Instance::resolve(def, &args) {
            Ok(instance) if instance.has_body() => Some(instance),
            _ => None,
        }
    };

    // First pass: the same primitive candidate for every type parameter (the common case,
    // and cheap). Second pass: the cartesian product of the per-parameter candidate lists,
    // capped at GENERIC_INSTANTIATION_ATTEMPT_LIMIT trait-solver queries, which finds
    // instantiations for functions whose parameters need *different* types (e.g.
    // `fn cast<T: Float, U: PrimInt>`) or types implementing non-primitive-friendly bounds.
    for candidate in generic_instantiation_candidates() {
        if let Some(instance) = try_choice(&vec![candidate; type_slots.len()]) {
            return Ok(instance);
        }
    }
    if !type_slots.is_empty() {
        let mut odometer = vec![0usize; type_slots.len()];
        'product: loop {
            let mut choice: Vec<Ty> =
                odometer.iter().enumerate().map(|(i, &c)| slot_candidates[i][c]).collect();
            let deferred_ok = resolve_deferred_fn_slots(
                tcx,
                &identity_args,
                &type_slots,
                &mut choice,
                &deferred_fn,
                nondet_fns,
            );
            // Skip choices already tried in the uniform pass.
            let uniform = choice.iter().all(|ty| *ty == choice[0])
                && generic_instantiation_candidates().contains(&choice[0]);
            if !uniform && deferred_ok {
                if let Some(instance) = try_choice(&choice) {
                    return Ok(instance);
                }
                if attempts.get() >= GENERIC_INSTANTIATION_ATTEMPT_LIMIT {
                    break;
                }
            }
            // Advance the odometer.
            for i in (0..odometer.len()).rev() {
                odometer[i] += 1;
                if odometer[i] < slot_candidates[i].len() {
                    continue 'product;
                }
                odometer[i] = 0;
                if i == 0 {
                    break 'product;
                }
            }
        }
    }
    Err(format!(
        "no candidate type ({}{}) satisfies the function's trait bounds",
        generic_instantiation_candidates()
            .iter()
            .map(|ty| ty.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        if n_impl_derived > 0 {
            format!(" and {n_impl_derived} types implementing the required traits")
        } else {
            String::new()
        }
    ))
}

/// Partition every function in the crate into (chosen, skipped), where `chosen` is a vector of the Instances for which we'll generate automatic harnesses,
/// and `skipped` is a map of function names to the reason why we skipped them.
fn automatic_harness_partition(
    tcx: TyCtxt,
    args: &Arguments,
    crate_name: &str,
    kani_any_def: FnDef,
    nondet_fns: &NondetFnModels,
) -> (Vec<Instance>, BTreeMap<String, AutoHarnessSkipReason>) {
    let crate_fn_defs = rustc_public::local_crate().fn_defs().into_iter().collect::<FxHashSet<_>>();
    // Filter out CrateItems that are functions, but not functions defined in the crate itself, i.e., rustc-inserted functions
    // (c.f. https://github.com/model-checking/kani/issues/4189)
    let crate_fns = rustc_public::all_local_items().into_iter().filter(|item| {
        if let TyKind::RigidTy(RigidTy::FnDef(def, _)) = item.ty().kind() {
            crate_fn_defs.contains(&def)
        } else {
            false
        }
    });

    let included_set = make_regex_set(args.autoharness_included_patterns.clone());
    let excluded_set = make_regex_set(args.autoharness_excluded_patterns.clone());

    // Cache whether a type implements or can derive Arbitrary
    let mut ty_arbitrary_cache: FxHashMap<Ty, bool> = FxHashMap::default();

    // If `instance` is not eligible for an automatic harness, return the reason why; if it is eligible, return None.
    // Note that we only return one reason for ineligiblity, when there could be multiple;
    // we can revisit this implementation choice in the future if users request more verbose output.
    let mut skip_reason = |instance: Instance| -> Option<AutoHarnessSkipReason> {
        if !instance.has_body() {
            return Some(AutoHarnessSkipReason::NoBody);
        }

        // Preprend the crate name so that users can filter out entire crates using the existing function filter flags.
        // `instance.name()` is already crate-qualified for local items (rust-lang/rust#149401),
        // so strip that prefix first to avoid double-qualifying (`crate::crate::fn`).
        let name = format!(
            "{crate_name}::{}",
            crate::kani_middle::strip_local_crate_prefix(instance.name())
        );
        let body = instance.body().unwrap();

        if is_proof_harness(tcx, instance)
            || name.contains("kani::Arbitrary")
            || name.contains("kani::Invariant")
        {
            return Some(AutoHarnessSkipReason::KaniImpl);
        }

        if autoharness_filtered_out(&name, &included_set, &excluded_set) {
            return Some(AutoHarnessSkipReason::UserFilter);
        }

        // Each argument of `instance` must implement Arbitrary.
        // Note that generic functions have been instantiated with concrete types at this point,
        // so we know that each of these arguments has a concrete type.
        let mut problematic_args = vec![];
        for (idx, arg) in body.arg_locals().iter().enumerate() {
            // Function items (Fn-bound instantiations, c.f. fn_bound_candidates) are
            // zero-sized values materialized as constants; no Arbitrary impl is involved.
            if matches!(arg.ty.kind(), TyKind::RigidTy(RigidTy::FnDef(..))) {
                continue;
            }
            if !ty_arbitrary_cache.contains_key(&arg.ty) {
                let impls_arbitrary =
                    implements_arbitrary(arg.ty, kani_any_def, &mut ty_arbitrary_cache)
                        || can_derive_arbitrary(arg.ty, kani_any_def, &mut ty_arbitrary_cache);
                ty_arbitrary_cache.insert(arg.ty, impls_arbitrary);
            }
            let impls_arbitrary = ty_arbitrary_cache.get(&arg.ty).unwrap();

            if !impls_arbitrary {
                // Find the name of the argument by referencing var_debug_info.
                // Note that enumerate() starts at 0, while rustc_public argument_index starts at 1, hence the idx+1.
                let arg_name = body
                    .var_debug_info
                    .iter()
                    .find(|var| {
                        var.argument_index.is_some_and(|arg_idx| idx + 1 == usize::from(arg_idx))
                    })
                    .map_or("_".to_string(), |debug_info| debug_info.name.to_string());
                let arg_type = format!("{}", arg.ty);
                problematic_args.push((arg_name, arg_type))
            }
        }
        if !problematic_args.is_empty() {
            return Some(AutoHarnessSkipReason::MissingArbitraryImpl(problematic_args));
        }
        None
    };

    let mut chosen = vec![];
    let mut skipped = BTreeMap::new();

    for func in crate_fns {
        if KaniAttributes::for_def_id(tcx, func.def_id()).is_kani_instrumentation() {
            skipped.insert(
                crate::kani_middle::strip_local_crate_prefix(func.name()),
                AutoHarnessSkipReason::KaniImpl,
            );
            continue;
        }

        // For generic functions, try to find a monomorphic instantiation whose bounds are
        // satisfied; the generated harness verifies the function for that instantiation only,
        // and its name (e.g. `foo::<i32>`) reflects that.
        let instance = match Instance::try_from(func) {
            Ok(instance) => instance,
            Err(_) => match choose_generic_instantiation(tcx, func, nondet_fns) {
                Ok(instance) => instance,
                Err(detail) => {
                    skipped.insert(
                        crate::kani_middle::strip_local_crate_prefix(func.name()),
                        AutoHarnessSkipReason::GenericFn(detail),
                    );
                    continue;
                }
            },
        };

        if let Some(reason) = skip_reason(instance) {
            skipped.insert(crate::kani_middle::strip_local_crate_prefix(instance.name()), reason);
        } else {
            chosen.push(instance);
        }
    }

    (chosen, skipped)
}

#[cfg(test)]
mod autoharness_filter_tests {
    use super::*;

    #[test]
    fn both_none() {
        let included = None;
        let excluded = None;
        assert!(!autoharness_filtered_out("test_fn", &included, &excluded));
    }

    #[test]
    fn only_included() {
        let included = make_regex_set(vec!["test.*".to_string()]);
        let excluded = None;

        assert!(!autoharness_filtered_out("test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("other_fn", &included, &excluded));
    }

    #[test]
    fn only_excluded() {
        let included = None;
        let excluded = make_regex_set(vec!["test.*".to_string()]);

        assert!(autoharness_filtered_out("test_fn", &included, &excluded));
        assert!(!autoharness_filtered_out("other_fn", &included, &excluded));
    }

    #[test]
    fn both_matching() {
        let included = make_regex_set(vec![".*_fn".to_string()]);
        let excluded = make_regex_set(vec!["test.*".to_string()]);

        assert!(autoharness_filtered_out("test_fn", &included, &excluded));
        assert!(!autoharness_filtered_out("other_fn", &included, &excluded));
    }

    #[test]
    fn multiple_include_patterns() {
        let included = make_regex_set(vec!["test.*".to_string(), "other.*".to_string()]);
        let excluded = None;

        assert!(!autoharness_filtered_out("test_fn", &included, &excluded));
        assert!(!autoharness_filtered_out("other_fn", &included, &excluded));
        assert!(autoharness_filtered_out("different_fn", &included, &excluded));
    }

    #[test]
    fn multiple_exclude_patterns() {
        let included = None;
        let excluded = make_regex_set(vec!["test.*".to_string(), "other.*".to_string()]);

        assert!(autoharness_filtered_out("test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("other_fn", &included, &excluded));
        assert!(!autoharness_filtered_out("different_fn", &included, &excluded));
    }

    #[test]
    fn exclude_precedence_identical_patterns() {
        let pattern = "test.*".to_string();
        let included = make_regex_set(vec![pattern.clone()]);
        let excluded = make_regex_set(vec![pattern]);

        assert!(autoharness_filtered_out("test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("other_fn", &included, &excluded));
    }

    #[test]
    fn exclude_precedence_overlapping_patterns() {
        let included = make_regex_set(vec![".*_fn".to_string()]);
        let excluded = make_regex_set(vec!["test_.*".to_string(), "other_.*".to_string()]);

        assert!(autoharness_filtered_out("test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("other_fn", &included, &excluded));
        assert!(!autoharness_filtered_out("different_fn", &included, &excluded));
    }

    #[test]
    fn exact_match() {
        let included = make_regex_set(vec!["^test_fn$".to_string()]);
        let excluded = None;

        assert!(!autoharness_filtered_out("test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("test_fn_extra", &included, &excluded));
    }

    #[test]
    fn include_specific_module() {
        let included = make_regex_set(vec!["module1::.*".to_string()]);
        let excluded = None;

        assert!(!autoharness_filtered_out("module1::test_fn", &included, &excluded));
        assert!(!autoharness_filtered_out("crate::module1::test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("module2::test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("crate::module2::test_fn", &included, &excluded));
    }

    #[test]
    fn exclude_specific_module() {
        let included = None;
        let excluded = make_regex_set(vec![".*::internal::.*".to_string()]);

        assert!(autoharness_filtered_out("crate::internal::helper_fn", &included, &excluded));
        assert!(autoharness_filtered_out("my_crate::internal::test_fn", &included, &excluded));
        assert!(!autoharness_filtered_out("crate::public::test_fn", &included, &excluded));
    }

    #[test]
    fn test_exact_match_with_crate() {
        let included = make_regex_set(vec!["^lib::foo_function$".to_string()]);
        let excluded = None;

        assert!(!autoharness_filtered_out("lib::foo_function", &included, &excluded));
        assert!(autoharness_filtered_out("lib::foo_function_extra", &included, &excluded));
        assert!(autoharness_filtered_out("lib::other::foo_function", &included, &excluded));
        assert!(autoharness_filtered_out("other::foo_function", &included, &excluded));
        assert!(autoharness_filtered_out("foo_function", &included, &excluded));
    }

    #[test]
    fn complex_path_patterns() {
        let included = make_regex_set(vec![
            "crate::module1::.*".to_string(),
            "other_crate::tests::.*".to_string(),
        ]);
        let excluded =
            make_regex_set(vec![".*::internal::.*".to_string(), ".*::private::.*".to_string()]);

        assert!(!autoharness_filtered_out("crate::module1::test_fn", &included, &excluded));
        assert!(!autoharness_filtered_out("other_crate::tests::test_fn", &included, &excluded));
        assert!(autoharness_filtered_out(
            "crate::module1::internal::test_fn",
            &included,
            &excluded
        ));
        assert!(autoharness_filtered_out(
            "other_crate::tests::private::test_fn",
            &included,
            &excluded
        ));
        assert!(autoharness_filtered_out("crate::module2::test_fn", &included, &excluded));
    }

    #[test]
    fn crate_specific_filtering() {
        let included = make_regex_set(vec!["my_crate::.*".to_string()]);
        let excluded = make_regex_set(vec!["other_crate::.*".to_string()]);

        assert!(!autoharness_filtered_out("my_crate::test_fn", &included, &excluded));
        assert!(!autoharness_filtered_out("my_crate::module::test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("other_crate::test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("third_crate::test_fn", &included, &excluded));
    }

    #[test]
    fn root_crate_paths() {
        let included = make_regex_set(vec!["^crate::.*".to_string()]);
        let excluded = None;

        assert!(!autoharness_filtered_out("crate::test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("other_crate::test_fn", &included, &excluded));
        assert!(autoharness_filtered_out("test_fn", &included, &excluded));
    }

    #[test]
    fn impl_paths_with_spaces() {
        let included = make_regex_set(vec!["num::<impl.i8>::wrapping_.*".to_string()]);
        let excluded = None;

        assert!(!autoharness_filtered_out("num::<impl i8>::wrapping_sh", &included, &excluded));
        assert!(!autoharness_filtered_out("num::<impl i8>::wrapping_add", &included, &excluded));
        assert!(autoharness_filtered_out("num::<impl i16>::wrapping_sh", &included, &excluded));
    }
}
