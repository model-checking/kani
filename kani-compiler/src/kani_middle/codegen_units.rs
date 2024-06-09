// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module is responsible for extracting grouping harnesses that can be processed together
//! by codegen.
//!
//! Today, only stub / contracts can affect the harness codegen. Thus, we group the harnesses
//! according to their stub configuration.

// TODO: Move this out of CBMC crate.
use crate::args::ReachabilityType;
use crate::kani_middle::attributes::is_proof_harness;
use crate::kani_middle::metadata::gen_proof_metadata;
use crate::kani_middle::reachability::filter_crate_items;
use crate::kani_middle::stubbing::{check_compatibility, harness_stub_map};
use crate::kani_queries::QueryDb;
use cbmc::{InternString, InternedString};
use kani_metadata::{ArtifactType, HarnessMetadata, KaniMetadata};
use rustc_hir::def_id::{DefId, DefPathHash};
use rustc_middle::ty::TyCtxt;
use rustc_session::config::OutputType;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::ty::{FnDef, RigidTy, TyKind};
use stable_mir::CrateDef;
use std::collections::{BTreeMap, HashMap};
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
    /// The metadata output path that shall be generated as part of the crate compilation.
    pub output_path: PathBuf,
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
        let crate_info = CrateInfo {
            name: stable_mir::local_crate().name.as_str().into(),
            output_path: metadata_output_path(tcx),
        };
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

    pub fn store_metadata(&self, queries: &QueryDb, tcx: TyCtxt) {
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
            self.harness_info.values().cloned().partition(|md| md.attributes.proof);
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

fn validate_units(tcx: TyCtxt, units: &[CodegenUnit]) {
    for unit in units {
        for (from, to) in &unit.stubs {
            let Err(msg) = check_compatibility(tcx, *from, *to) else { continue };
            let span = unit.harnesses.first().unwrap().def.span();
            tcx.dcx().span_err(rustc_internal::internal(tcx, span), msg);
        }
    }
    tcx.dcx().abort_if_errors();
}
