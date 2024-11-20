// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module handles outputting the result for the list subcommand

use std::{
    fmt::Display,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use crate::{args::list_args::Format, list::ListMetadata, version::KANI_VERSION};
use anyhow::Result;
use comfy_table::Table as PrettyTable;
use serde_json::json;
use to_markdown_table::MarkdownTable;

// Represents the version of our JSON file format.
// Increment this version (according to semantic versioning rules) whenever the JSON output format changes.
const FILE_VERSION: &str = "0.1";
const OUTPUT_FILENAME: &str = "kani-list";

/// Output the results of the list subcommand.
pub fn output_list_results(list_metadata: ListMetadata, format: Format, quiet: bool) -> Result<()> {
    match format {
        Format::Pretty => pretty(list_metadata),
        Format::Markdown => markdown(list_metadata, quiet),
        Format::Json => json(list_metadata, quiet),
    }
}

/// Print results to the terminal.
fn pretty(list_metadata: ListMetadata) -> Result<()> {
    let table = if list_metadata.contracted_functions.is_empty() {
        None
    } else {
        let (header, rows) = construct_contracts_table(&list_metadata);
        let mut t = PrettyTable::new();
        t.set_header(header).add_rows(rows);
        Some(t)
    };
    let output = format_results(table, &list_metadata);
    println!("{}", output);

    Ok(())
}

/// Output results to a Markdown file.
fn markdown(list_metadata: ListMetadata, quiet: bool) -> Result<()> {
    let table = if list_metadata.contracted_functions.is_empty() {
        None
    } else {
        let (header, rows) = construct_contracts_table(&list_metadata);
        Some(MarkdownTable::new(Some(header), rows)?)
    };

    let output = format_results(table, &list_metadata);

    let out_path = Path::new(OUTPUT_FILENAME).with_extension("md");
    let mut out_file = File::create(&out_path).unwrap();
    out_file.write_all(output.as_bytes()).unwrap();
    if !quiet {
        println!("Wrote list results to {}", std::fs::canonicalize(&out_path)?.display());
    }
    Ok(())
}

/// Output results as a JSON file.
fn json(list_metadata: ListMetadata, quiet: bool) -> Result<()> {
    let out_path = Path::new(OUTPUT_FILENAME).with_extension("json");
    let out_file = File::create(&out_path).unwrap();
    let writer = BufWriter::new(out_file);

    let json_obj = json!({
        "kani-version": KANI_VERSION,
        "file-version": FILE_VERSION,
        "standard-harnesses": &list_metadata.standard_harnesses,
        "contract-harnesses": &list_metadata.contract_harnesses,
        "contracts": &list_metadata.contracted_functions,
        "totals": {
            "standard-harnesses": list_metadata.standard_harnesses_count,
            "contract-harnesses": list_metadata.contract_harnesses_count,
            "functions-under-contract": list_metadata.contracted_functions.len(),
        }
    });

    serde_json::to_writer_pretty(writer, &json_obj)?;

    if !quiet {
        println!("Wrote list results to {}", std::fs::canonicalize(out_path)?.display());
    }

    Ok(())
}

/// Construct the rows for the table of contracts information.
/// Returns a tuple of the table header and the rows.
fn construct_contracts_table(list_metadata: &ListMetadata) -> (Vec<String>, Vec<Vec<String>>) {
    const NO_HARNESSES_MSG: &str = "NONE";
    const FUNCTION_HEADER: &str = "Function";
    const CONTRACT_HARNESSES_HEADER: &str = "Contract Harnesses (#[kani::proof_for_contract])";
    const TOTALS_HEADER: &str = "Total";

    let header =
        vec![String::new(), FUNCTION_HEADER.to_string(), CONTRACT_HARNESSES_HEADER.to_string()];

    let mut rows: Vec<Vec<String>> = vec![];

    for cf in &list_metadata.contracted_functions {
        let mut row = vec![String::new(), cf.function.clone()];
        if cf.harnesses.is_empty() {
            row.push(NO_HARNESSES_MSG.to_string());
        } else {
            row.push(cf.harnesses.join(", "));
        }
        rows.push(row);
    }

    let totals_row = vec![
        TOTALS_HEADER.to_string(),
        list_metadata.contracted_functions.len().to_string(),
        list_metadata.contract_harnesses_count.to_string(),
    ];
    rows.push(totals_row);

    (header, rows)
}

/// Format results as a String
fn format_results<T: Display>(table: Option<T>, list_metadata: &ListMetadata) -> String {
    const CONTRACTS_SECTION: &str = "Contracts:";
    const HARNESSES_SECTION: &str = "Standard Harnesses (#[kani::proof]):";
    const NO_CONTRACTS_MSG: &str = "No contracts or contract harnesses found.";
    const NO_HARNESSES_MSG: &str = "No standard harnesses found.";

    let mut output: Vec<String> = vec![];
    output.push(format!("\n{CONTRACTS_SECTION}"));

    if let Some(table) = table {
        output.push(format!("{table}"));
    } else {
        output.push(NO_CONTRACTS_MSG.to_string());
    }

    output.push(format!("\n{HARNESSES_SECTION}"));
    if list_metadata.standard_harnesses.is_empty() {
        output.push(NO_HARNESSES_MSG.to_string());
    }

    let mut std_harness_index = 0;

    for harnesses in (&list_metadata.standard_harnesses).values() {
        for harness in harnesses {
            output.push(format!("{}. {harness}", std_harness_index + 1));
            std_harness_index += 1;
        }
    }

    output.join("\n")
}
