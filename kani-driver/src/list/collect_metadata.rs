// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This module invokes the compiler to gather the metadata for the list subcommand, then post-processes the output.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    InvocationType,
    args::{
        VerificationArgs,
        list_args::{CargoListArgs, StandaloneListArgs},
    },
    list::ListMetadata,
    list::output::output_list_results,
    project::{Project, cargo_project, standalone_project, std_project},
    session::KaniSession,
    version::print_kani_version,
};
use anyhow::Result;
use kani_metadata::{ContractedFunction, HarnessKind, KaniMetadata};

/// Process the KaniMetadata output from kani-compiler and output the list subcommand results
pub fn process_metadata(metadata: Vec<KaniMetadata>) -> ListMetadata {
    // We use ordered maps and sets so that the output is in lexicographic order (and consistent across invocations).

    // Map each file to a vector of its harnesses.
    let mut standard_harnesses: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut contract_harnesses: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    let mut contracted_functions: BTreeSet<ContractedFunction> = BTreeSet::new();

    let mut standard_harnesses_count = 0;
    let mut contract_harnesses_count = 0;

    for kani_meta in metadata {
        for harness_meta in kani_meta.proof_harnesses {
            match harness_meta.attributes.kind {
                HarnessKind::Proof => {
                    standard_harnesses_count += 1;
                    if let Some(harnesses) = standard_harnesses.get_mut(&harness_meta.original_file)
                    {
                        harnesses.insert(harness_meta.pretty_name);
                    } else {
                        standard_harnesses.insert(
                            harness_meta.original_file,
                            BTreeSet::from([harness_meta.pretty_name]),
                        );
                    }
                }
                HarnessKind::ProofForContract { .. } => {
                    contract_harnesses_count += 1;
                    if let Some(harnesses) = contract_harnesses.get_mut(&harness_meta.original_file)
                    {
                        harnesses.insert(harness_meta.pretty_name);
                    } else {
                        contract_harnesses.insert(
                            harness_meta.original_file,
                            BTreeSet::from([harness_meta.pretty_name]),
                        );
                    }
                }
                HarnessKind::Test => {}
            }
        }

        contracted_functions.extend(kani_meta.contracted_functions.into_iter());
    }

    ListMetadata {
        standard_harnesses,
        standard_harnesses_count,
        contract_harnesses,
        contract_harnesses_count,
        contracted_functions,
    }
}

pub fn list_cargo(args: CargoListArgs, mut verify_opts: VerificationArgs) -> Result<()> {
    let quiet = args.common_args.quiet;
    verify_opts.common_args = args.common_args;
    let mut session = KaniSession::new(verify_opts)?;
    if !quiet {
        print_kani_version(InvocationType::CargoKani(vec![]));
    }

    let project = cargo_project(&mut session, false)?;
    let list_metadata = process_metadata(project.metadata);

    output_list_results(list_metadata, args.format, quiet)
}

pub fn list_standalone(args: StandaloneListArgs, mut verify_opts: VerificationArgs) -> Result<()> {
    let quiet = args.common_args.quiet;
    verify_opts.common_args = args.common_args;
    let session = KaniSession::new(verify_opts)?;
    if !quiet {
        print_kani_version(InvocationType::Standalone);
    }

    let project: Project = if args.std {
        std_project(&args.input, &session)?
    } else {
        standalone_project(&args.input, args.crate_name, &session)?
    };

    let list_metadata = process_metadata(project.metadata);

    output_list_results(list_metadata, args.format, quiet)
}
