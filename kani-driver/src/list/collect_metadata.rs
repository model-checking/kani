use crate::{
    args::list_args::{CargoListArgs, Format, StandaloneListArgs},
    project::{cargo_project, std_project},
    session::{KaniSession, ReachabilityMode},
    version::print_kani_version,
    InvocationType,
};
use anyhow::Result;
use kani_metadata::{ContractedFunction, HarnessKind, KaniMetadata};

use super::output::{json, pretty};

fn process_metadata(metadata: Vec<KaniMetadata>, format: Format) -> Result<()> {
    let mut standard_harnesses: Vec<String> = vec![];
    let mut contract_harnesses: Vec<String> = vec![];
    let mut contracted_functions: Vec<ContractedFunction> = vec![];
    let mut total_contracts = 0;

    for kani_meta in metadata {
        for harness_meta in kani_meta.proof_harnesses {
            match harness_meta.attributes.kind {
                HarnessKind::Proof => standard_harnesses.push(harness_meta.pretty_name),
                HarnessKind::ProofForContract { .. } => {
                    contract_harnesses.push(harness_meta.pretty_name)
                }
                HarnessKind::Test => {}
            }
        }

        for cf in &kani_meta.contracted_functions {
            total_contracts += cf.total_contracts;
        }

        contracted_functions.extend(kani_meta.contracted_functions.into_iter());
    }

    // Print in alphabetical order
    standard_harnesses.sort();
    contract_harnesses.sort();
    contracted_functions.sort_by_key(|cf| cf.function.clone());

    match format {
        Format::Pretty => pretty(
            standard_harnesses,
            contracted_functions,
            contract_harnesses.len(),
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
    //let project = standalone_project(&args.input, args.crate_name, &session)?;
    let project = std_project(&args.input, &session)?;

    process_metadata(project.metadata, args.format)
}

pub fn _list_std(_session: KaniSession, _args: CargoListArgs) -> Result<()> {
    todo!()
}
