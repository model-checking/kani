// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module handles outputting the result for the list subcommand

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    fs::File,
    io::BufWriter,
};

use crate::{args::list_args::Format, list::Totals, version::KANI_VERSION};
use anyhow::Result;
use colour::print_ln_bold;
use comfy_table::Table as PrettyTable;
use kani_metadata::ContractedFunction;
use serde_json::json;
use to_markdown_table::MarkdownTable;

// Represents the version of our JSON file format.
// Increment this version (according to semantic versioning rules) whenever the JSON output format changes.
const FILE_VERSION: &str = "0.1";
const JSON_FILENAME: &str = "kani-list.json";

/// Construct the rows for the table of contracts information.
/// Returns a tuple of the table header and the rows.
fn construct_contracts_table(
    contracted_functions: BTreeSet<ContractedFunction>,
    totals: Totals,
) -> (Vec<String>, Vec<Vec<String>>) {
    const NO_HARNESSES_MSG: &str = "NONE";
    const FUNCTION_HEADER: &str = "Function";
    const CONTRACT_HARNESSES_HEADER: &str = "Contract Harnesses (#[kani::proof_for_contract])";
    const TOTALS_HEADER: &str = "Total";

    let header =
        vec![String::new(), FUNCTION_HEADER.to_string(), CONTRACT_HARNESSES_HEADER.to_string()];

    let mut rows: Vec<Vec<String>> = vec![];

    for cf in contracted_functions {
        let mut row = vec![String::new(), cf.function];
        if cf.harnesses.is_empty() {
            row.push(NO_HARNESSES_MSG.to_string());
        } else {
            row.push(cf.harnesses.join(", "));
        }
        rows.push(row);
    }

    let totals_row = vec![
        TOTALS_HEADER.to_string(),
        totals.contracted_functions.to_string(),
        totals.contract_harnesses.to_string(),
    ];
    rows.push(totals_row);

    (header, rows)
}

/// Print results to the terminal.
fn print_results<T: Display>(
    table: Option<T>,
    standard_harnesses: BTreeMap<String, BTreeSet<String>>,
) {
    const CONTRACTS_SECTION: &str = "Contracts:";
    const HARNESSES_SECTION: &str = "Standard Harnesses (#[kani::proof]):";
    const NO_CONTRACTS_MSG: &str = "No contracts or contract harnesses found.";
    const NO_HARNESSES_MSG: &str = "No standard harnesses found.";

    print_ln_bold!("\n{CONTRACTS_SECTION}");

    if let Some(table) = table {
        println!("{table}");
    } else {
        println!("{NO_CONTRACTS_MSG}");
    }

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
}

/// Output results as a JSON file.
fn json(
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

    println!("Wrote list results to {}", std::fs::canonicalize(JSON_FILENAME)?.display());

    Ok(())
}

/// Output the results of the list subcommand.
pub fn output_list_results(
    standard_harnesses: BTreeMap<String, BTreeSet<String>>,
    contract_harnesses: BTreeMap<String, BTreeSet<String>>,
    contracted_functions: BTreeSet<ContractedFunction>,
    totals: Totals,
    format: Format,
) -> Result<()> {
    match format {
        Format::Pretty => {
            let table = if totals.contracted_functions == 0 {
                None
            } else {
                let (header, rows) = construct_contracts_table(contracted_functions, totals);
                let mut t = PrettyTable::new();
                t.set_header(header).add_rows(rows);
                Some(t)
            };
            print_results(table, standard_harnesses);
            Ok(())
        }
        Format::Markdown => {
            let table = if totals.contracted_functions == 0 {
                None
            } else {
                let (header, rows) = construct_contracts_table(contracted_functions, totals);
                Some(MarkdownTable::new(Some(header), rows)?)
            };
            print_results(table, standard_harnesses);
            Ok(())
        }
        Format::Json => json(standard_harnesses, contract_harnesses, contracted_functions, totals),
    }
}
