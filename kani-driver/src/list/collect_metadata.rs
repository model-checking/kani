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
    list::output::output_list_results,
    list::{FileName, HarnessName, ListMetadata},
    project::{Project, cargo_project, standalone_project, std_project},
    session::KaniSession,
    version::print_kani_version,
};
use anyhow::Result;
use kani_metadata::{ContractedFunction, HarnessKind, HarnessMetadata, KaniMetadata};

/// Process the KaniMetadata output from kani-compiler and output the list subcommand results
pub fn process_metadata(metadata: Vec<KaniMetadata>) -> BTreeSet<ListMetadata> {
    let mut list_metadata: BTreeSet<ListMetadata> = BTreeSet::new();

    let insert = |harness_meta: HarnessMetadata,
                  map: &mut BTreeMap<FileName, BTreeSet<HarnessName>>,
                  count: &mut usize| {
        *count += 1;
        if let Some(harnesses) = map.get_mut(&harness_meta.original_file) {
            harnesses.insert(harness_meta.pretty_name);
        } else {
            map.insert(harness_meta.original_file, BTreeSet::from([harness_meta.pretty_name]));
        };
    };

    for kani_meta in metadata {
        // We use ordered maps and sets so that the output is in lexicographic order (and consistent across invocations).
        let mut standard_harnesses: BTreeMap<FileName, BTreeSet<HarnessName>> = BTreeMap::new();
        let mut contract_harnesses: BTreeMap<FileName, BTreeSet<HarnessName>> = BTreeMap::new();
        let mut contracted_functions: BTreeSet<ContractedFunction> = BTreeSet::new();

        let mut standard_harnesses_count = 0;
        let mut contract_harnesses_count = 0;

        for harness_meta in kani_meta.proof_harnesses {
            match harness_meta.attributes.kind {
                HarnessKind::Proof => {
                    insert(harness_meta, &mut standard_harnesses, &mut standard_harnesses_count);
                }
                HarnessKind::ProofForContract { .. } => {
                    insert(harness_meta, &mut contract_harnesses, &mut contract_harnesses_count);
                }
                HarnessKind::Test => {}
            }
        }

        contracted_functions.extend(kani_meta.contracted_functions.into_iter());

        list_metadata.insert(ListMetadata {
            crate_name: kani_meta.crate_name,
            standard_harnesses,
            standard_harnesses_count,
            contract_harnesses,
            contract_harnesses_count,
            contracted_functions,
        });
    }

    list_metadata
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
