// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::session::KaniSession;
use crate::util::alter_extension;
use anyhow::Result;
use kani_metadata::HarnessMetadata;
use serde_json::Value;

impl KaniSession {
    /// Extract deterministic values from a failing harness.
    pub fn get_det_vals(&self, file: &Path, harness_metadata: &HarnessMetadata) -> Result<()> {
        let results_filename = alter_extension(file, "results.json");
        let det_vals_filename = alter_extension(file, "det_vals.txt");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(results_filename.clone());
            temps.push(det_vals_filename.clone());
        }

        self.cbmc_variant(file, &["--trace", "--json-ui"], &results_filename, harness_metadata)?;

        // Let the user know
        if !self.args.quiet {
            println!("Extracting deterministic values from trace");
        }

        let cbmc_out = get_cbmc_out(results_filename);
        let det_vals = handle_cbmc_out(&cbmc_out);
        let joined_det_vals = det_vals.join("\n");
        if self.args.debug {
            println!("Det vals:\n{}", joined_det_vals);
        }
        fs::write(det_vals_filename, joined_det_vals).unwrap();

        Ok(())
    }
}

fn get_cbmc_out(results_filename: PathBuf) -> Value {
    let results_file = File::open(results_filename).unwrap();
    let reader = BufReader::new(results_file);
    let cbmc_out: Value = serde_json::from_reader(reader).unwrap();
    return cbmc_out;
}

fn handle_cbmc_out(cbmc_out: &Value) -> Vec<String> {
    let mut det_vals: Vec<String> = Vec::new();
    for general_msg in cbmc_out.as_array().unwrap() {
        let result_msg = &general_msg["result"];
        if !result_msg.is_null() {
            for result_val in result_msg.as_array().unwrap() {
                let mut det_vals_for_result = handle_result(result_val);
                det_vals.append(&mut det_vals_for_result);
            }
        }
    }
    return det_vals;
}

fn handle_result(result_val: &Value) -> Vec<String> {
    let mut det_vals: Vec<String> = Vec::new();
    let desc = result_val["description"].as_str().unwrap();

    if desc.contains("assertion failed") {
        for trace_val in result_val["trace"].as_array().unwrap() {
            let mut det_vals_for_trace = handle_trace(trace_val);
            det_vals.append(&mut det_vals_for_trace);
        }
    }

    return det_vals;
}

fn handle_trace(trace_val: &Value) -> Vec<String> {
    let mut det_vals: Vec<String> = Vec::new();
    let step_type = &trace_val["stepType"];
    if step_type != "assignment" {
        return det_vals;
    }

    let lhs = trace_val["lhs"].as_str().unwrap();
    if lhs != "non_det_byte_arr" {
        return det_vals;
    }

    let func = trace_val["sourceLocation"]["function"].as_str().unwrap();
    if func != "kani::any_raw" {
        return det_vals;
    }

    let members_list = trace_val["value"]["members"].as_array().unwrap();
    let byte_arr = members_list[0]["value"]["elements"].as_array().unwrap();

    for a_byte in byte_arr {
        let data = &a_byte["value"]["data"];
        let file_line = format!("{}", data);
        det_vals.push(file_line);
    }

    return det_vals;
}
