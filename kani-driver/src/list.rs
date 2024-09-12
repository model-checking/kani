// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Implements the list subcommand logic

use std::{fs::File, io::BufWriter};

use crate::{args::list_args::{CargoListArgs, Format, StandaloneListArgs}, metadata::from_json, project::{self, Artifact}, session::{KaniSession, ReachabilityMode}, util::crate_name, version::{print_kani_version, KANI_VERSION}, InvocationType};
use anyhow::Result;
use cli_table::{print_stdout, Cell, CellStruct, Style, Table};
use colour::print_ln_bold;
use kani_metadata::{ArtifactType, ContractedFunction, HarnessKind, KaniMetadata};
use serde_json::json;

// Represents the version of our JSON file format.
// Increment this version (according to semantic versioning rules) whenever the JSON output format changes.
const FILE_VERSION: &str = "0.1";

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
            total_contracts += cf.total_contracts;
        }

        contracted_functions.extend(kani_meta.contracted_functions.into_iter());
    }

    // Print in alphabetical order
    standard_harnesses.sort();
    contract_harnesses.sort();
    contracted_functions.sort_by_key(|cf| cf.function.clone());

    match format {
        Format::Pretty => pretty_print(standard_harnesses, contracted_functions, contract_harnesses.len(), total_contracts),
        Format::Json => json(standard_harnesses, contract_harnesses, contracted_functions, total_contracts),
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

    // TODO delete intermediate files

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
    standard_harnesses: Vec<String>,
    contracted_functions: Vec<ContractedFunction>,
    total_contract_harnesses: usize,
    total_contracts: usize,
) -> Result<()> {
    let total_contracted_functions = contracted_functions.len();

    fn format_contract_harnesses(harnesses: &mut Vec<String>) -> String {
        harnesses.sort();
        let joined = harnesses.join("\n");
        if joined.is_empty() {
            "NONE".to_string()
        } else {
            joined
        }
    }

    let mut contracts_table: Vec<Vec<CellStruct>> = vec![];

    for mut cf in contracted_functions {
        contracts_table.push(vec!["".cell(), cf.function.cell(), cf.total_contracts.cell(), format_contract_harnesses(&mut cf.harnesses).cell()]);
    }

    contracts_table.push(vec!["Total".cell().bold(true), total_contracted_functions.cell(), total_contracts.cell(), total_contract_harnesses.cell()]);

    print_ln_bold!("\nContracts:");
    print_stdout(contracts_table.table().title(vec![
        "".cell(),
        "Function Under Contract".cell().bold(true),
        "# of Contracts".cell().bold(true),
        "Contract Harnesses".cell().bold(true),
        ])
    )?;

    print_ln_bold!("\nStandard Harnesses:");
    for (i, harness) in standard_harnesses.iter().enumerate() {
        println!("{}. {harness}", i+1);
    }

    Ok(())
}

fn json(
    standard_harnesses: Vec<String>,
    contract_harnesses: Vec<String>,
    contracted_functions: Vec<ContractedFunction>,
    total_contracts: usize,
) -> Result<()> {
    // FIXME
    let filename = "list.json";

    let out_file = File::create(filename).unwrap();
    let writer = BufWriter::new(out_file);

    let json_obj = json!({
        "kani-version": KANI_VERSION,
        "file-version": FILE_VERSION,
        "standard-harnesses": &standard_harnesses,
        "contract-harnesses": &contract_harnesses,
        "contracts": &contracted_functions,
        "totals": {
            "standard-harnesses": standard_harnesses.len(),
            "contract-harnesses": contract_harnesses.len(),
            "functions-under-contract": contracted_functions.len(),
            "contracts": total_contracts,
        }
    });
    
    serde_json::to_writer_pretty(writer, &json_obj)?;

    Ok(())
}