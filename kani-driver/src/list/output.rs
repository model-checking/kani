// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This module handles outputting the result for the list subcommand

use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::BufWriter,
};

use crate::{list::Totals, version::KANI_VERSION};
use anyhow::Result;
use colour::print_ln_bold;
use kani_metadata::ContractedFunction;
use serde_json::json;

// Represents the version of our JSON file format.
// Increment this version (according to semantic versioning rules) whenever the JSON output format changes.
const FILE_VERSION: &str = "0.1";
const JSON_FILENAME: &str = "kani-list.json";

/// Construct the table of contracts information.
fn construct_contracts_table(
    contracted_functions: BTreeSet<ContractedFunction>,
    totals: Totals,
) -> Vec<String> {
    const NO_HARNESSES_MSG: &str = "NONE";

    // Since the harnesses will be separated by newlines, the harness length is equal to the length of the longest harness
    fn harnesses_len(harnesses: &[String]) -> usize {
        harnesses.iter().map(|s| s.len()).max().unwrap_or(NO_HARNESSES_MSG.len())
    }

    // Contracts table headers
    const FUNCTION_HEADER: &str = "Function";
    const CONTRACTS_COUNT_HEADER: &str = "# of Contracts";
    const CONTRACT_HARNESSES_HEADER: &str = "Contract Harnesses (#[kani::proof_for_contract])";

    // Contracts table totals row
    const TOTALS_HEADER: &str = "Total";
    let functions_total = totals.contracted_functions.to_string();
    let contracts_total = totals.contracts.to_string();
    let harnesses_total = totals.contract_harnesses.to_string();

    let mut table_rows: Vec<String> = vec![];
    let mut max_function_fmt_width = max(FUNCTION_HEADER.len(), functions_total.len());
    let mut max_contracts_count_fmt_width =
        max(CONTRACTS_COUNT_HEADER.len(), contracts_total.len());
    let mut max_contract_harnesses_fmt_width =
        max(CONTRACT_HARNESSES_HEADER.len(), harnesses_total.len());

    let mut data_rows: Vec<(String, String, Vec<String>)> = vec![];

    for cf in contracted_functions {
        max_function_fmt_width = max(max_function_fmt_width, cf.function.len());
        max_contracts_count_fmt_width = max(max_contracts_count_fmt_width, cf.total_contracts);
        max_contract_harnesses_fmt_width =
            max(max_contract_harnesses_fmt_width, harnesses_len(&cf.harnesses));

        data_rows.push((cf.function, cf.total_contracts.to_string(), cf.harnesses));
    }

    let function_sep = "-".repeat(max_function_fmt_width);
    let contracts_count_sep = "-".repeat(max_contracts_count_fmt_width);
    let contract_harnesses_sep = "-".repeat(max_contract_harnesses_fmt_width);
    let totals_sep = "-".repeat(TOTALS_HEADER.len());

    let sep_row = format!(
        "| {totals_sep} | {function_sep} | {contracts_count_sep} | {contract_harnesses_sep} |"
    );
    table_rows.push(sep_row.clone());

    let function_space = " ".repeat(max_function_fmt_width - FUNCTION_HEADER.len());
    let contracts_count_space =
        " ".repeat(max_contracts_count_fmt_width - CONTRACTS_COUNT_HEADER.len());
    let contract_harnesses_space =
        " ".repeat(max_contract_harnesses_fmt_width - CONTRACT_HARNESSES_HEADER.len());
    let totals_space = " ".repeat(TOTALS_HEADER.len());

    let header_row = format!(
        "| {totals_space} | {FUNCTION_HEADER}{function_space} | {CONTRACTS_COUNT_HEADER}{contracts_count_space} | {CONTRACT_HARNESSES_HEADER}{contract_harnesses_space} |"
    );
    table_rows.push(header_row);
    table_rows.push(sep_row.clone());

    for (function, total_contracts, harnesses) in data_rows {
        let function_space = " ".repeat(max_function_fmt_width - function.len());
        let contracts_count_space =
            " ".repeat(max_contracts_count_fmt_width - total_contracts.len());
        let first_harness = harnesses.first().map_or(NO_HARNESSES_MSG, |v| v);
        let contract_harnesses_space =
            " ".repeat(max_contract_harnesses_fmt_width - first_harness.len());

        let first_row = format!(
            "| {totals_space} | {function}{function_space} | {total_contracts}{contracts_count_space} | {first_harness}{contract_harnesses_space} |"
        );
        table_rows.push(first_row);

        for subsequent_harness in harnesses.iter().skip(1) {
            let function_space = " ".repeat(max_function_fmt_width);
            let contracts_count_space = " ".repeat(max_contracts_count_fmt_width);
            let contract_harnesses_space =
                " ".repeat(max_contract_harnesses_fmt_width - subsequent_harness.len());
            let row = format!(
                "| {totals_space} | {function_space} | {contracts_count_space} | {subsequent_harness}{contract_harnesses_space} |"
            );
            table_rows.push(row);
        }

        table_rows.push(sep_row.clone())
    }

    let total_function_space = " ".repeat(max_function_fmt_width - functions_total.len());
    let total_contracts_space = " ".repeat(max_contracts_count_fmt_width - contracts_total.len());
    let total_harnesses_space =
        " ".repeat(max_contract_harnesses_fmt_width - harnesses_total.len());

    let totals_row = format!(
        "| {TOTALS_HEADER} | {functions_total}{total_function_space} | {contracts_total}{total_contracts_space} | {harnesses_total}{total_harnesses_space} |"
    );

    table_rows.push(totals_row);
    table_rows.push(sep_row.clone());

    table_rows
}

/// Output results as a table printed to the terminal.
pub fn pretty(
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
        let table_rows = construct_contracts_table(contracted_functions, totals);
        println!("{}", table_rows.join("\n"));
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
            "contracts": totals.contracts,
        }
    });

    serde_json::to_writer_pretty(writer, &json_obj)?;

    Ok(())
}
