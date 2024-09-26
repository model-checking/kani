// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This module invokes the compiler to gather the metadata for the list subcommand, then post-processes the output.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    args::list_args::{CargoListArgs, Format, StandaloneListArgs},
    list::output::{json, pretty},
    list::Totals,
    project::{cargo_project, standalone_project, std_project, Project},
    session::KaniSession,
    version::print_kani_version,
    InvocationType,
};
use anyhow::Result;
use kani_metadata::{ContractedFunction, HarnessKind, KaniMetadata};

fn process_metadata(metadata: Vec<KaniMetadata>, format: Format) -> Result<()> {
    // Map each file to a vector of its harnesses
    let mut standard_harnesses: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut contract_harnesses: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut contracted_functions: BTreeSet<ContractedFunction> = BTreeSet::new();

    let mut total_standard_harnesses = 0;
    let mut total_contract_harnesses = 0;
    let mut total_contracts = 0;

    for kani_meta in metadata {
        for harness_meta in kani_meta.proof_harnesses {
            match harness_meta.attributes.kind {
                HarnessKind::Proof => {
                    total_standard_harnesses += 1;
                    if let Some(harnesses) = standard_harnesses.get_mut(&harness_meta.original_file)
                    {
                        harnesses.insert(harness_meta.pretty_name);
                    } else {
                        standard_harnesses
                            .insert(harness_meta.original_file, BTreeSet::from([harness_meta.pretty_name]));
                    }
                }
                HarnessKind::ProofForContract { .. } => {
                    total_contract_harnesses += 1;
                    if let Some(harnesses) = contract_harnesses.get_mut(&harness_meta.original_file)
                    {
                        harnesses.insert(harness_meta.pretty_name);
                    } else {
                        contract_harnesses
                            .insert(harness_meta.original_file, BTreeSet::from([harness_meta.pretty_name]));
                    }
                }
                HarnessKind::Test => {}
            }
        }

        for cf in &kani_meta.contracted_functions {
            total_contracts += cf.total_contracts;
        }

        contracted_functions.extend(kani_meta.contracted_functions.into_iter());
    }

    let totals = Totals {
        standard_harnesses: total_standard_harnesses,
        contract_harnesses: total_contract_harnesses,
        contracted_functions: contracted_functions.len(),
        contracts: total_contracts,
    };

    match format {
        Format::Pretty => pretty(standard_harnesses, contracted_functions, totals),
        Format::Json => json(standard_harnesses, contract_harnesses, contracted_functions, totals),
    }
}

pub fn list_cargo(args: CargoListArgs) -> Result<()> {
    let session = KaniSession::new(args.verify_opts)?;
    if !session.args.common_args.quiet {
        print_kani_version(InvocationType::CargoKani(vec![]));
    }

    let project = cargo_project(&session, false)?;
    process_metadata(project.metadata, args.format)
}

pub fn list_standalone(args: StandaloneListArgs) -> Result<()> {
    let session = KaniSession::new(args.verify_opts)?;
    if !session.args.common_args.quiet {
        print_kani_version(InvocationType::Standalone);
    }

    let project: Project = if args.std {
        std_project(&args.input, &session)?
    } else {
        standalone_project(&args.input, args.crate_name, &session)?
    };

    process_metadata(project.metadata, args.format)
}
