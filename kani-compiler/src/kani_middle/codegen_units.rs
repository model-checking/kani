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
use crate::kani_middle::stubbing::harness_stub_map;
use crate::kani_queries::QueryDb;
use cbmc::{InternString, InternedString};
use kani_metadata::{ArtifactType, HarnessMetadata, KaniMetadata};
use rustc_hir::def_id::{DefId, DefPathHash};
use rustc_middle::ty::TyCtxt;
use rustc_session::config::OutputType;
use stable_mir::mir::mono::Instance;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use tracing::debug;

/// A stable (across compilation sessions) identifier for the harness function.
type Harness = Instance;

/// A set of stubs.
type Stubs = HashMap<DefId, DefId>;

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

/// We group the harnesses that have the same stubs.
pub struct CodegenUnits {
    units: Vec<Vec<Harness>>,
    all_harnesses: HashMap<Harness, HarnessInfo>,
    crate_info: CrateInfo,
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
                    let stub_map = harness_stub_map(tcx, harness, &metadata);
                    (harness, HarnessInfo { metadata, stub_map })
                })
                .collect::<HashMap<_, _>>();

            let (no_stubs, with_stubs): (Vec<_>, Vec<_>) = if queries.args().stubbing_enabled {
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
            let mut units = vec![no_stubs];
            units.extend(group_by_stubs(tcx, with_stubs, &all_harnesses));
            CodegenUnits { units, all_harnesses, crate_info }
        } else {
            // Leave other reachability type handling as is for now.
            CodegenUnits { units: vec![], all_harnesses: HashMap::default(), crate_info }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vec<Harness>> {
        self.units.iter()
    }

    pub fn store_metadata(&self, queries: &QueryDb, tcx: TyCtxt) {
        let metadata = self.generate_metadata();
        let outpath = metadata_output_path(tcx);
        store_metadata(queries, &metadata, &outpath);
    }

    pub fn harness_model_path(&self, harness: Harness) -> Option<&PathBuf> {
        self.all_harnesses[&harness].metadata.goto_file.as_ref()
    }

    /// Generate [KaniMetadata] for the target crate.
    fn generate_metadata(&self) -> KaniMetadata {
        let (proof_harnesses, test_harnesses) = self
            .all_harnesses
            .values()
            .map(|info| &info.metadata)
            .cloned()
            .partition(|md| md.attributes.proof);
        KaniMetadata {
            crate_name: self.crate_info.name.clone(),
            proof_harnesses,
            unsupported_features: vec![],
            test_harnesses,
        }
    }
}

/// Group the harnesses by their stubs.
fn group_by_stubs(
    tcx: TyCtxt,
    harnesses: Vec<Harness>,
    all_harnesses: &HashMap<Harness, HarnessInfo>,
) -> Vec<Vec<Harness>> {
    let mut per_stubs: HashMap<BTreeMap<DefPathHash, DefPathHash>, Vec<Harness>> =
        HashMap::default();
    for harness in harnesses {
        let stub_map = all_harnesses[&harness]
            .stub_map
            .iter()
            .map(|(k, v)| (tcx.def_path_hash(*k), tcx.def_path_hash(*v)))
            .collect::<BTreeMap<_, _>>();
        per_stubs.entry(stub_map).or_default().push(harness)
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
