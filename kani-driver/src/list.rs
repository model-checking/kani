// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Implements the list subcommand logic

use std::collections::BTreeMap;

use crate::{args::list_args::{CargoListArgs, Format, StandaloneListArgs}, metadata::from_json, project::{self, Artifact}, session::{KaniSession, ReachabilityMode}, util::crate_name, version::print_kani_version, InvocationType};
use anyhow::Result;
use cli_table::{print_stdout, Cell, Style, Table};
use colour::print_ln_bold;
use kani_metadata::{ArtifactType, ContractedFunction, HarnessKind, KaniMetadata};

fn set_session_args(session: &mut KaniSession) {
    session.reachability_mode = ReachabilityMode::None;
    session.args.list_enabled = true;
}

fn process_metadata(metadata: Vec<KaniMetadata>, format: Format) -> Result<()> {
    let mut standard_harnesses: Vec<String> = vec![];
    let mut contract_harnesses: Vec<String> = vec![];
    let mut contracted_functions: Vec<ContractedFunction> = vec![];
    let mut total_contracts = 0;
    
    for kani_meta in metadata {
        for harness_meta in kani_meta.proof_harnesses {
            match harness_meta.attributes.kind {
                HarnessKind::Proof => standard_harnesses.push(harness_meta.pretty_name),
                HarnessKind::ProofForContract { .. } => contract_harnesses.push(harness_meta.pretty_name),
                HarnessKind::Test => {}
            }
        }

        for cf in &kani_meta.contracted_functions {
            total_contracts += cf.contracts_count;
        }

        contracted_functions.extend(kani_meta.contracted_functions.into_iter());
    }

    let totals = BTreeMap::from([
        ("Standard Harnesses", standard_harnesses.len()),
        ("Contract Harnesses", contract_harnesses.len()),
        ("Functions with Contracts", contracted_functions.len()),
        ("Contracts", total_contracts)
    ]);

    match format {
        Format::Pretty => pretty_print(standard_harnesses, contract_harnesses, contracted_functions, totals),
        Format::Json => json_print(standard_harnesses, contract_harnesses, contracted_functions, totals),
    }
}

pub fn list_cargo(mut session: KaniSession, args: CargoListArgs) -> Result<()> {
    set_session_args(&mut session);
    let project = project::cargo_project(&session, false)?;
    // process_project(project, args.format)
    todo!()
}

pub fn list_standalone(args: StandaloneListArgs) -> Result<()> {
    let mut session = KaniSession::new(args.verify_opts)?;
    if !session.args.common_args.quiet {
        print_kani_version(InvocationType::Standalone);
    }
    set_session_args(&mut session);
    
    let crate_name = if let Some(name) = args.crate_name { name } else { crate_name(&args.input) };

    // Ensure the directory exist and it's in its canonical form.
    let outdir = if let Some(target_dir) = &session.args.target_dir {
        std::fs::create_dir_all(target_dir)?; // This is a no-op if directory exists.
        target_dir.canonicalize()?
    } else {
        args.input.canonicalize().unwrap().parent().unwrap().to_path_buf()
    };

    session.compile_single_rust_file(&args.input, &crate_name, &outdir)?;

    let mut path = outdir.join(crate_name.clone());
    let _ = path.set_extension(ArtifactType::Metadata);
    let m = Artifact::try_new(&path, ArtifactType::Metadata)?;

    let metadata: KaniMetadata = from_json(&m)?;

    process_metadata(vec![metadata], args.format)
}

pub fn list_std(session: KaniSession, args: CargoListArgs) -> Result<()> {
    todo!()
}

fn pretty_print(
    mut standard_harnesses: Vec<String>,
    mut contract_harnesses: Vec<String>,
    mut contracted_functions: Vec<ContractedFunction>,
    totals: BTreeMap<&str, usize>
) -> Result<()> {

    standard_harnesses.sort();
    contract_harnesses.sort();
    contracted_functions.sort_by_key(|cf| cf.pretty_name.clone());

    print_ln_bold!("\nContracts:");
    
    print_stdout(
        contracted_functions
        .table()
        .title(vec![
            "Function".cell().bold(true),
            "# of Contracts".cell().bold(true),
            "Contract Harnesses".cell().bold(true),
        ])
    )?;


    print_ln_bold!("\nContract Harnesses:");
    for harness in &contract_harnesses {
        println!("- {}", harness);
    }

    print_ln_bold!("\nStandard Harnesses:");
    for harness in &standard_harnesses {
        println!("- {}", harness);
    }

    print_ln_bold!("\nTotals:");
    for (key, total) in totals {
        println!("{key}: {total}");
    }

    Ok(())
}

fn json_print(
    standard_harnesses: Vec<String>,
    contract_harnesses: Vec<String>,
    contracted_functions: Vec<ContractedFunction>,
    totals: BTreeMap<&str, usize>
) -> Result<()> {
    todo!()
}