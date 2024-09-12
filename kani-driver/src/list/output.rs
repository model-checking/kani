use std::{fs::File, io::BufWriter};

use crate::version::KANI_VERSION;
use anyhow::Result;
use cli_table::{print_stdout, Cell, CellStruct, Style, Table};
use colour::print_ln_bold;
use kani_metadata::ContractedFunction;
use serde_json::json;

// Represents the version of our JSON file format.
// Increment this version (according to semantic versioning rules) whenever the JSON output format changes.
const FILE_VERSION: &str = "0.1";

pub fn pretty(
    standard_harnesses: Vec<String>,
    contracted_functions: Vec<ContractedFunction>,
    total_contract_harnesses: usize,
    total_contracts: usize,
) -> Result<()> {
    let total_contracted_functions = contracted_functions.len();

    fn format_contract_harnesses(harnesses: &mut [String]) -> String {
        harnesses.sort();
        let joined = harnesses.join("\n");
        if joined.is_empty() { "NONE".to_string() } else { joined }
    }

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
        total_contracted_functions.cell(),
        total_contracts.cell(),
        total_contract_harnesses.cell(),
    ]);

    print_ln_bold!("\nContracts:");
    print_stdout(contracts_table.table().title(vec![
        "".cell(),
        "Function Under Contract".cell().bold(true),
        "# of Contracts".cell().bold(true),
        "Contract Harnesses".cell().bold(true),
    ]))?;

    print_ln_bold!("\nStandard Harnesses:");
    for (i, harness) in standard_harnesses.iter().enumerate() {
        println!("{}. {harness}", i + 1);
    }

    Ok(())
}

pub fn json(
    standard_harnesses: Vec<String>,
    contract_harnesses: Vec<String>,
    contracted_functions: Vec<ContractedFunction>,
    total_contracts: usize,
) -> Result<()> {
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
