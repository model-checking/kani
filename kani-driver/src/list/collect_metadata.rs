// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This module invokes the compiler to gather the metadata for the list subcommand, then post-processes the output.

use std::collections::BTreeMap;

use crate::{
    args::list_args::{CargoListArgs, Format, StandaloneListArgs},
    project::{cargo_project, standalone_project, std_project, Project},
    session::{KaniSession, ReachabilityMode},
    version::print_kani_version,
    InvocationType,
};
use anyhow::Result;
use kani_metadata::{ContractedFunction, HarnessKind, KaniMetadata};

use super::output::{json, pretty};

fn process_metadata(metadata: Vec<KaniMetadata>, format: Format) -> Result<()> {
    // Map each file to a vector of its harnesses
    let mut standard_harnesses: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut contract_harnesses: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut contracted_functions: Vec<ContractedFunction> = vec![];
    let mut total_contracts = 0;
    let mut total_contract_harnesses = 0;

    for kani_meta in metadata {
        for harness_meta in kani_meta.proof_harnesses {
            match harness_meta.attributes.kind {
                HarnessKind::Proof => {
                    if let Some(harnesses) = standard_harnesses.get_mut(&harness_meta.original_file)
                    {
                        harnesses.push(harness_meta.pretty_name);
                    } else {
                        standard_harnesses
                            .insert(harness_meta.original_file, vec![harness_meta.pretty_name]);
                    }
                }
                HarnessKind::ProofForContract { .. } => {
                    if let Some(harnesses) = contract_harnesses.get_mut(&harness_meta.original_file)
                    {
                        harnesses.push(harness_meta.pretty_name);
                    } else {
                        contract_harnesses
                            .insert(harness_meta.original_file, vec![harness_meta.pretty_name]);
                    }
                }
                HarnessKind::Test => {}
            }
        }

        for cf in &kani_meta.contracted_functions {
            total_contract_harnesses += cf.harnesses.len();
            total_contracts += cf.total_contracts;
        }

        contracted_functions.extend(kani_meta.contracted_functions.into_iter());
    }

    // Print in alphabetical order
    contracted_functions.sort_by_key(|cf| cf.function.clone());

    match format {
        Format::Pretty => pretty(
            standard_harnesses,
            contracted_functions,
            total_contract_harnesses,
            total_contracts,
        ),
        Format::Json => {
            json(standard_harnesses, contract_harnesses, contracted_functions, total_contracts)
        }
    }
}

pub fn list_cargo(args: CargoListArgs) -> Result<()> {
    let mut session = KaniSession::new(args.verify_opts)?;
    if !session.args.common_args.quiet {
        print_kani_version(InvocationType::CargoKani(vec![]));
    }
    session.reachability_mode = ReachabilityMode::None;

    let project = cargo_project(&session, false)?;
    process_metadata(project.metadata, args.format)
}

pub fn list_standalone(args: StandaloneListArgs) -> Result<()> {
    let mut session = KaniSession::new(args.verify_opts)?;
    if !session.args.common_args.quiet {
        print_kani_version(InvocationType::Standalone);
    }
    session.reachability_mode = ReachabilityMode::None;

    let project: Project = if args.std {
        std_project(&args.input, &session)?
    } else {
        standalone_project(&args.input, args.crate_name, &session)?
    };

    process_metadata(project.metadata, args.format)
}
