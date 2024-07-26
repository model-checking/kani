// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module is responsible for extracting grouping harnesses that can be processed together
//! by codegen.
//!
//! Today, only stub / contracts can affect the harness codegen. Thus, we group the harnesses
//! according to their stub configuration.

use crate::args::ReachabilityType;
use crate::kani_middle::attributes::is_proof_harness;
use crate::kani_middle::metadata::gen_proof_metadata;
use crate::kani_middle::reachability::filter_crate_items;
use crate::kani_middle::stubbing::{check_compatibility, harness_stub_map};
use crate::kani_queries::QueryDb;
use kani_metadata::{ArtifactType, AssignsContract, HarnessMetadata, KaniMetadata};
use rustc_hir::def_id::{DefId, DefPathHash};
use rustc_middle::ty::TyCtxt;
use rustc_session::config::OutputType;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::ty::{FnDef, RigidTy, TyKind};
use stable_mir::CrateDef;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use tracing::debug;

/// A stable (across compilation sessions) identifier for the harness function.
type Harness = Instance;

/// A set of stubs.
pub type Stubs = HashMap<FnDef, FnDef>;

/// Store some relevant information about the crate compilation.
#[derive(Clone, Debug)]
struct CrateInfo {
    /// The name of the crate being compiled.
    pub name: String,
}

/// We group the harnesses that have the same stubs.
pub struct CodegenUnits {
    units: Vec<CodegenUnit>,
    harness_info: HashMap<Harness, HarnessMetadata>,
    crate_info: CrateInfo,
}

#[derive(Clone, Default, Debug)]
pub struct CodegenUnit {
    pub harnesses: Vec<Harness>,
    pub stubs: Stubs,
}

impl CodegenUnits {
    pub fn new(queries: &QueryDb, tcx: TyCtxt) -> Self {
        let crate_info = CrateInfo { name: stable_mir::local_crate().name.as_str().into() };
        if queries.args().reachability_analysis == ReachabilityType::Harnesses {
            let base_filepath = tcx.output_filenames(()).path(OutputType::Object);
            let base_filename = base_filepath.as_path();
            let harnesses = filter_crate_items(tcx, |_, instance| is_proof_harness(tcx, instance));
            let all_harnesses = harnesses
                .into_iter()
                .map(|harness| {
                    let metadata = gen_proof_metadata(tcx, harness, &base_filename);
                    (harness, metadata)
                })
                .collect::<HashMap<_, _>>();

            // Even if no_stubs is empty we still need to store rustc metadata.
            let units = group_by_stubs(tcx, &all_harnesses);
            validate_units(tcx, &units);
            debug!(?units, "CodegenUnits::new");
            CodegenUnits { units, harness_info: all_harnesses, crate_info }
        } else {
            // Leave other reachability type handling as is for now.
            CodegenUnits { units: vec![], harness_info: HashMap::default(), crate_info }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &CodegenUnit> {
        self.units.iter()
    }

    /// We store which instance of modifies was generated.
    pub fn store_modifies(&mut self, harness_modifies: &[(Harness, AssignsContract)]) {
        for (harness, modifies) in harness_modifies {
            self.harness_info.get_mut(harness).unwrap().contract = Some(modifies.clone());
        }
    }

    /// Write compilation metadata into a file.
    pub fn write_metadata(&self, queries: &QueryDb, tcx: TyCtxt) {
        let metadata = self.generate_metadata();
        let outpath = metadata_output_path(tcx);
        store_metadata(queries, &metadata, &outpath);
    }

    pub fn harness_model_path(&self, harness: Harness) -> Option<&PathBuf> {
        self.harness_info[&harness].goto_file.as_ref()
    }

    /// Generate [KaniMetadata] for the target crate.
    fn generate_metadata(&self) -> KaniMetadata {
        let (proof_harnesses, test_harnesses) =
            self.harness_info.values().cloned().partition(|md| md.attributes.is_proof());
        KaniMetadata {
            crate_name: self.crate_info.name.clone(),
            proof_harnesses,
            unsupported_features: vec![],
            test_harnesses,
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

/// Group the harnesses by their stubs.
fn group_by_stubs(
    tcx: TyCtxt,
    all_harnesses: &HashMap<Harness, HarnessMetadata>,
) -> Vec<CodegenUnit> {
    let mut per_stubs: HashMap<BTreeMap<DefPathHash, DefPathHash>, CodegenUnit> =
        HashMap::default();
    for (harness, metadata) in all_harnesses {
        let stub_ids = harness_stub_map(tcx, *harness, metadata);
        let stub_map = stub_ids
            .iter()
            .map(|(k, v)| (tcx.def_path_hash(*k), tcx.def_path_hash(*v)))
            .collect::<BTreeMap<_, _>>();
        if let Some(unit) = per_stubs.get_mut(&stub_map) {
            unit.harnesses.push(*harness);
        } else {
            let stubs = stub_ids
                .iter()
                .map(|(from, to)| (stub_def(tcx, *from), stub_def(tcx, *to)))
                .collect::<HashMap<_, _>>();
            let stubs = apply_transitivity(tcx, *harness, stubs);
            per_stubs.insert(stub_map, CodegenUnit { stubs, harnesses: vec![*harness] });
        }
    }
    per_stubs.into_values().collect()
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
                        harness.def.name(),
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
