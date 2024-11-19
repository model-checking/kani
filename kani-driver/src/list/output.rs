// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module handles outputting the result for the list subcommand

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::BufWriter,
};

use crate::{list::Totals, version::KANI_VERSION};
use anyhow::Result;
use colour::print_ln_bold;
use kani_metadata::ContractedFunction;
use serde_json::json;
use to_markdown_table::{MarkdownTable, MarkdownTableError};

// Represents the version of our JSON file format.
// Increment this version (according to semantic versioning rules) whenever the JSON output format changes.
const FILE_VERSION: &str = "0.1";
const JSON_FILENAME: &str = "kani-list.json";

/// Construct the table of contracts information.
fn construct_contracts_table(
    contracted_functions: BTreeSet<ContractedFunction>,
    totals: Totals,
) -> Result<MarkdownTable, MarkdownTableError> {
    const NO_HARNESSES_MSG: &str = "NONE";
    const FUNCTION_HEADER: &str = "Function";
    const CONTRACT_HARNESSES_HEADER: &str = "Contract Harnesses (#[kani::proof_for_contract])";
    const TOTALS_HEADER: &str = "Total";

    let mut table_rows: Vec<Vec<String>> = vec![];

    for cf in contracted_functions {
        let first_harness = cf.harnesses.first().map_or(NO_HARNESSES_MSG, |v| v).to_string();
        let first_row = vec![String::new(), cf.function.clone(), first_harness];
        table_rows.push(first_row);

        for subsequent_harness in cf.harnesses.iter().skip(1) {
            let row = vec![String::new(), cf.function.clone(), subsequent_harness.to_string()];
            table_rows.push(row);
        }
    }

    let totals_row = vec![
        TOTALS_HEADER.to_string(),
        totals.contracted_functions.to_string(),
        totals.contract_harnesses.to_string(),
    ];
    table_rows.push(totals_row);

    let table =
        MarkdownTable::new(Some(vec!["", FUNCTION_HEADER, CONTRACT_HARNESSES_HEADER]), table_rows)?;

    Ok(table)
}

// /// Output results as a table printed to the terminal.
pub fn markdown(
    standard_harnesses: BTreeMap<String, BTreeSet<String>>,
    contracted_functions: BTreeSet<ContractedFunction>,
    totals: Totals,
) -> Result<()> {
    const CONTRACTS_SECTION: &str = "Contracts:";
    const HARNESSES_SECTION: &str = "Standard Harnesses (#[kani::proof]):";
    const NO_CONTRACTS_MSG: &str = "No contracts or contract harnesses found.";
    const NO_HARNESSES_MSG: &str = "No standard harnesses found.";

    print_ln_bold!("\n{CONTRACTS_SECTION}");

    if contracted_functions.is_empty() {
        println!("{NO_CONTRACTS_MSG}");
    } else {
        let table = construct_contracts_table(contracted_functions, totals)?;
        println!("{}", table);
    };

    print_ln_bold!("\n{HARNESSES_SECTION}");
    if standard_harnesses.is_empty() {
        println!("{NO_HARNESSES_MSG}");
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

/// Output results as a JSON file.
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
        }
    });

    serde_json::to_writer_pretty(writer, &json_obj)?;

    Ok(())
}
