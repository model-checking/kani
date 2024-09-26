// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This module handles outputting the result for the list subcommand

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::BufWriter,
};

use crate::{list::Totals, version::KANI_VERSION};
use anyhow::Result;
use cli_table::{print_stdout, Cell, CellStruct, Style, Table};
use colour::print_ln_bold;
use kani_metadata::ContractedFunction;
use serde_json::json;

// Represents the version of our JSON file format.
// Increment this version (according to semantic versioning rules) whenever the JSON output format changes.
const FILE_VERSION: &str = "0.1";
const JSON_FILENAME: &str = "kani-list.json";

pub fn pretty(
    standard_harnesses: BTreeMap<String, BTreeSet<String>>,
    contracted_functions: BTreeSet<ContractedFunction>,
    totals: Totals,
) -> Result<()> {
    fn format_contract_harnesses(harnesses: &mut [String]) -> String {
        harnesses.sort();
        let joined = harnesses.join("\n");
        if joined.is_empty() { "NONE".to_string() } else { joined }
    }

    print_ln_bold!("\nContracts:");
    println!(
        "Each function in the table below either has contracts or is the target of a contract harness (#[kani::proof_for_contract])."
    );

    if contracted_functions.is_empty() {
        println!("No contracts or contract harnesses found.")
    } else {
        let mut contracts_table: Vec<Vec<CellStruct>> = vec![];

        for mut cf in contracted_functions {
            contracts_table.push(vec![
                "".cell(),
                cf.function.cell(),
                cf.total_contracts.cell(),
                format_contract_harnesses(&mut cf.harnesses).cell(),
            ]);
        }

        contracts_table.push(vec![
            "Total".cell().bold(true),
            totals.contracted_functions.cell(),
            totals.contracts.cell(),
            totals.contract_harnesses.cell(),
        ]);

        print_stdout(contracts_table.table().title(vec![
            "".cell(),
            "Function".cell().bold(true),
            "# of Contracts".cell().bold(true),
            "Contract Harnesses".cell().bold(true),
        ]))?;
    }

    print_ln_bold!("\nStandard Harnesses (#[kani::proof]):");
    if standard_harnesses.is_empty() {
        println!("No standard harnesses found.");
    }

    let mut std_harness_index = 0;

    for (_, harnesses) in standard_harnesses {
        for harness in harnesses {
            println!("{}. {harness}", std_harness_index + 1);
            std_harness_index += 1;
        }
    }

    println!();

    Ok(())
}

pub fn json(
    standard_harnesses: BTreeMap<String, BTreeSet<String>>,
    contract_harnesses: BTreeMap<String, BTreeSet<String>>,
    contracted_functions: BTreeSet<ContractedFunction>,
    totals: Totals,
) -> Result<()> {
    let out_file = File::create(JSON_FILENAME).unwrap();
    let writer = BufWriter::new(out_file);

    let json_obj = json!({
        "kani-version": KANI_VERSION,
        "file-version": FILE_VERSION,
        "standard-harnesses": &standard_harnesses,
        "contract-harnesses": &contract_harnesses,
        "contracts": &contracted_functions,
        "totals": {
            "standard-harnesses": totals.standard_harnesses,
            "contract-harnesses": totals.contract_harnesses,
            "functions-under-contract": totals.contracted_functions,
            "contracts": totals.contracts,
        }
    });

    serde_json::to_writer_pretty(writer, &json_obj)?;

    Ok(())
}
