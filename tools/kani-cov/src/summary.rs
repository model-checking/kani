// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

use anyhow::Result;
use tree_sitter::{Node, Parser};

use crate::{args::SummaryArgs, coverage::CombinedCoverageResults};

pub fn summary_main(args: &SummaryArgs) -> Result<()> {
    let mapfile = File::open(&args.mapfile)?;
    let reader = BufReader::new(mapfile);

    let covfile = File::open(&args.profile)?;
    let covreader = BufReader::new(covfile);
    let cov_results: CombinedCoverageResults =
        serde_json::from_reader(covreader).expect("could not load coverage results");

    println!("{cov_results:?}");

    let source_files: Vec<PathBuf> =
        serde_json::from_reader(reader).expect("could not parse coverage metadata");

    let mut function_info: Vec<FunctionCoverageInfo> = Vec::new();

    for file in source_files {
        let new_info = function_info_from_file(&file);
        function_info.extend(new_info);
    }

    Ok(())
}

pub fn validate_summary_args(_args: &SummaryArgs) -> Result<()> {
    Ok(())
}

#[derive(Debug)]
struct FunctionCoverageInfo {
    name: String,
    start: (usize, usize),
    end: (usize, usize),
    num_lines: usize,
}

fn function_info_from_file(filepath: &PathBuf) -> Vec<FunctionCoverageInfo> {
    let source_code = fs::read_to_string(filepath).expect("could not read source file");
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::language()).expect("Error loading Rust grammar");

    let tree = parser.parse(&source_code, None).expect("Failed to parse file");

    let mut cursor = tree.walk();
    let first_child_exists = cursor.goto_first_child();

    if !first_child_exists {
        return vec![];
    }

    let mut function_info: Vec<FunctionCoverageInfo> = Vec::new();

    if cursor.node().kind() == "function_item" {
        function_info.push(function_info_from_node(cursor.node(), source_code.as_bytes()))
    };

    while cursor.goto_next_sibling() {
        if cursor.node().kind() == "function_item" {
            function_info.push(function_info_from_node(cursor.node(), source_code.as_bytes()))
        }
    }

    function_info
}

fn function_info_from_node<'a>(node: Node, source: &'a [u8]) -> FunctionCoverageInfo {
    let name = node
        .child_by_field_name("name")
        .and_then(|name| name.utf8_text(source).ok())
        .expect("couldn't get function name")
        .to_string();
    let start = (node.start_position().row, node.start_position().column);
    let end = (node.end_position().row, node.end_position().column);
    let num_lines = end.0 - start.0 + 1;
    FunctionCoverageInfo { name, start, end, num_lines }
}
