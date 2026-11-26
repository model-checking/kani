// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module handles outputting the result for the list subcommand

use std::{
    collections::BTreeSet,
    fmt::Display,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::{
    args::list_args::Format,
    list::{ListMetadata, merge_list_metadata},
    project::Project,
    version::KANI_VERSION,
};
use anyhow::{Context, Result};
use comfy_table::Table as PrettyTable;
use serde_json::json;
use to_markdown_table::MarkdownTable;

// Represents the version of our JSON file format.
// Increment this version (according to semantic versioning rules) whenever the JSON output format changes.
const FILE_VERSION: &str = "0.1";
const OUTPUT_FILENAME: &str = "kani-list";

/// The directory for `kani-list.md` or `kani-list.json` when `--output` is not given.
/// Cargo projects use Kani's folder in the cargo target directory, the same folder the build uses.
/// Standalone and `--std` projects have no cargo target directory and use the current directory.
pub fn default_list_dir(project: &Project, target_dir: Option<&Path>) -> PathBuf {
    match &project.cargo_metadata {
        Some(cargo_metadata) => target_dir
            .map_or_else(|| cargo_metadata.target_directory.clone().into(), Path::to_path_buf)
            .join("kani"),
        None => PathBuf::from("."),
    }
}

/// Output the results of the list subcommand.
/// The `markdown` and `json` formats write `kani-list.md` or `kani-list.json` to `output_dir`,
/// creating it if needed.
pub fn output_list_results(
    list_metadata: BTreeSet<ListMetadata>,
    format: Format,
    output_dir: &Path,
    quiet: bool,
) -> Result<()> {
    match format {
        Format::Pretty => pretty(list_metadata),
        Format::Markdown => markdown(list_metadata, output_dir, quiet),
        Format::Json => json(list_metadata, output_dir, quiet),
    }
}

fn pretty_constructor(header: Vec<String>, rows: Vec<Vec<String>>) -> Result<PrettyTable> {
    let mut t = PrettyTable::new();
    t.set_header(header).add_rows(rows);
    Ok(t)
}

fn markdown_constructor(header: Vec<String>, rows: Vec<Vec<String>>) -> Result<MarkdownTable> {
    Ok(MarkdownTable::new(Some(header), rows)?)
}

/// Construct the "Contracts" and "Standard Harnesses" tables.
/// `table_constructor` is a function that, given the header and rows for the tables, creates a particular kind of table.
fn construct_output<T: Display>(
    list_metadata: BTreeSet<ListMetadata>,
    table_constructor: fn(Vec<String>, Vec<Vec<String>>) -> Result<T>,
) -> Result<(String, String)> {
    let contract_output = {
        const CONTRACTS_SECTION: &str = "Contracts:";
        const NO_CONTRACTS_MSG: &str = "No contracts or contract harnesses found.";
        let contract_table = if list_metadata.iter().all(|md| md.contracted_functions.is_empty()) {
            None
        } else {
            let (header, rows) = construct_contracts_table(&list_metadata);
            let t = table_constructor(header, rows)?;
            Some(t)
        };
        format_results(contract_table, CONTRACTS_SECTION.to_string(), NO_CONTRACTS_MSG.to_string())
    };
    let standard_output = {
        const HARNESSES_SECTION: &str = "Standard Harnesses (#[kani::proof]):";
        const NO_HARNESSES_MSG: &str = "No standard harnesses found.";
        let standard_table = {
            let (header, rows) = construct_standard_table(&list_metadata);
            let t = table_constructor(header, rows)?;
            Some(t)
        };
        format_results(standard_table, HARNESSES_SECTION.to_string(), NO_HARNESSES_MSG.to_string())
    };
    Ok((contract_output, standard_output))
}

/// Print results to the terminal.
fn pretty(list_metadata: BTreeSet<ListMetadata>) -> Result<()> {
    let (contract_output, standard_output) = construct_output(list_metadata, pretty_constructor)?;
    println!("{contract_output}");
    println!("{standard_output}");

    Ok(())
}

/// Output results to a Markdown file.
fn markdown(list_metadata: BTreeSet<ListMetadata>, output_dir: &Path, quiet: bool) -> Result<()> {
    let (contract_output, standard_output) = construct_output(list_metadata, markdown_constructor)?;

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!("Failed to create list output directory `{}`", output_dir.display())
    })?;
    let out_path = output_dir.join(OUTPUT_FILENAME).with_extension("md");
    let mut out_file = File::create(&out_path)
        .with_context(|| format!("Failed to create list output file `{}`", out_path.display()))?;
    out_file.write_all(contract_output.as_bytes()).unwrap();
    out_file.write_all(standard_output.as_bytes()).unwrap();
    if !quiet {
        println!("Wrote list results to {}", std::fs::canonicalize(&out_path)?.display());
    }
    Ok(())
}

/// Output results as a JSON file.
fn json(list_metadata: BTreeSet<ListMetadata>, output_dir: &Path, quiet: bool) -> Result<()> {
    std::fs::create_dir_all(output_dir).with_context(|| {
        format!("Failed to create list output directory `{}`", output_dir.display())
    })?;
    let out_path = output_dir.join(OUTPUT_FILENAME).with_extension("json");
    let out_file = File::create(&out_path)
        .with_context(|| format!("Failed to create list output file `{}`", out_path.display()))?;
    let writer = BufWriter::new(out_file);

    let combined_md = merge_list_metadata(list_metadata);

    let json_obj = json!({
        "kani-version": KANI_VERSION,
        "file-version": FILE_VERSION,
        "standard-harnesses": combined_md.standard_harnesses,
        "contract-harnesses": combined_md.contract_harnesses,
        "contracts": combined_md.contracted_functions,
        "totals": {
            "standard-harnesses": combined_md.standard_harnesses_count,
            "contract-harnesses": combined_md.contract_harnesses_count,
            "functions-under-contract": combined_md.contracted_functions.len(),
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
fn construct_contracts_table(
    list_metadata: &BTreeSet<ListMetadata>,
) -> (Vec<String>, Vec<Vec<String>>) {
    const NO_HARNESSES_MSG: &str = "NONE";
    const CRATE_NAME: &str = "Crate";
    const FUNCTION_HEADER: &str = "Function";
    const CONTRACT_HARNESSES_HEADER: &str = "Contract Harnesses (#[kani::proof_for_contract])";
    const TOTALS_HEADER: &str = "Total";

    let header = vec![
        String::new(),
        CRATE_NAME.to_string(),
        FUNCTION_HEADER.to_string(),
        CONTRACT_HARNESSES_HEADER.to_string(),
    ];

    let mut rows: Vec<Vec<String>> = vec![];
    let mut functions_under_contract_total = 0;
    let mut contract_harnesses_total = 0;

    for crate_md in list_metadata {
        for cf in &crate_md.contracted_functions {
            let mut row = vec![String::new(), crate_md.crate_name.to_string(), cf.function.clone()];
            if cf.harnesses.is_empty() {
                row.push(NO_HARNESSES_MSG.to_string());
            } else {
                row.push(cf.harnesses.join(", "));
            }
            rows.push(row);
        }
        functions_under_contract_total += crate_md.contracted_functions.len();
        contract_harnesses_total += crate_md.contract_harnesses_count;
    }

    let totals_row = vec![
        TOTALS_HEADER.to_string(),
        String::new(),
        functions_under_contract_total.to_string(),
        contract_harnesses_total.to_string(),
    ];
    rows.push(totals_row);

    (header, rows)
}

fn construct_standard_table(
    list_metadata: &BTreeSet<ListMetadata>,
) -> (Vec<String>, Vec<Vec<String>>) {
    const CRATE_NAME: &str = "Crate";
    const HARNESS_HEADER: &str = "Harness";
    const TOTALS_HEADER: &str = "Total";

    let header = vec![String::new(), CRATE_NAME.to_string(), HARNESS_HEADER.to_string()];

    let mut rows: Vec<Vec<String>> = vec![];

    let mut total = 0;

    for crate_md in list_metadata {
        for harnesses in crate_md.standard_harnesses.values() {
            for harness in harnesses {
                rows.push(vec![
                    String::new(),
                    crate_md.crate_name.to_string(),
                    harness.to_string(),
                ]);
            }
            total += harnesses.len();
        }
    }

    let totals_row = vec![TOTALS_HEADER.to_string(), String::new(), total.to_string()];
    rows.push(totals_row);

    (header, rows)
}

fn format_results<T: Display>(
    table: Option<T>,
    section_name: String,
    absent_name: String,
) -> String {
    let mut output: Vec<String> = vec![];
    output.push(format!("\n{section_name}"));

    if let Some(table) = table {
        output.push(format!("{table}"));
    } else {
        output.push(absent_name);
    }

    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_list_dir_is_current_dir_without_cargo() {
        assert_eq!(default_list_dir(&Project::default(), None), PathBuf::from("."));
    }
}
