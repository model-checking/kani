// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MI&T

use std::ffi::OsString;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::KaniSession;
use crate::util::alter_extension;
use anyhow::Result;
use kani_metadata::HarnessMetadata;
use serde_json::Value;

impl KaniSession {
    /// Extract deterministic values from a failing harness.
    pub fn get_det_vals(&self, file: &Path, harness_metadata: &HarnessMetadata) -> Result<Vec<u8>> {
        let results_filename = alter_extension(file, "results.json");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(results_filename.clone());
        }

        self.cbmc_variant(file, &["--trace", "--json-ui"], &results_filename, harness_metadata)?;

        // Let the user know
        if !self.args.quiet {
            println!("Extracting deterministic values from trace");
        }

        let cbmc_out = get_cbmc_out(results_filename);
        let det_vals = handle_cbmc_out(&cbmc_out);

        Ok(det_vals)
    }

    pub fn format_unit_test(&self, harness_name: &str, det_vals: &Vec<u8>) -> String {
        format!(
            "#[test]
            fn {}_exec_trace() {{
                kani::DET_VALS.with(|det_vals| {{
                    *det_vals.borrow_mut() = vec!{:?};
                }});
                {}();
            }}
            ",
            harness_name, det_vals, harness_name
        )
    }

    pub fn modify_src_code(
        &self,
        src_file_path: &str,
        proof_harness_line: usize,
        exec_trace: &str,
    ) {
        // Prep the source code and exec trace func.
        let src_as_str =
            fs::read_to_string(src_file_path).expect("Couldn't access proof harness source file");
        let mut src_lines = src_as_str.split('\n').collect::<Vec<&str>>();
        let mut exec_trace_lines = exec_trace.split('\n').collect::<Vec<&str>>();

        // Calc inserted indexes and line numbers into source code to rustfmt only those lines.
        // TODO: Why -2?
        let start_idx = proof_harness_line - 2;
        let start_line = start_idx + 1;
        // This excludes the final new line, since including that causes a rustfmt error.
        let end_line = start_line + exec_trace_lines.len() - 2;

        // Splice the exec trace func into the proof harness source code.
        let mut src_right = src_lines.split_off(start_idx);
        src_lines.append(&mut exec_trace_lines);
        src_lines.append(&mut src_right);
        let new_src = src_lines.join("\n");

        println!("Now modifying the proof harness source file");
        fs::write(src_file_path, new_src).expect("Couldn't write to proof harness source file");

        // Run rustfmt on just the inserted lines.
        let source_path = Path::new(src_file_path);
        let parent_dir = source_path
            .parent()
            .expect("Proof harness is not in a directory")
            .to_str()
            .expect("Couldn't convert proof harness directory from OsStr to str");
        let src_file = source_path
            .file_name()
            .expect("Couldn't get file name of source file")
            .to_str()
            .expect("Couldn't convert proof harness file name from OsStr to str");
        self.run_rustfmt(src_file, Some(parent_dir), Some(start_line), Some(end_line));
    }

    pub fn run_rustfmt(
        &self,
        src_file: &str,
        current_dir_opt: Option<&str>,
        start_line_opt: Option<usize>,
        end_line_opt: Option<usize>,
    ) {
        let mut cmd = Command::new("rustfmt");

        let mut args: Vec<OsString> = Vec::new();
        if let (Some(start_line), Some(end_line)) = (start_line_opt, end_line_opt) {
            // rustfmt `--file-lines` arg is currently unstable.
            args.push("--unstable-features".into());

            let file_line_arg =
                format!("[{{\"file\":\"{}\",\"range\":[{},{}]}}]", src_file, start_line, end_line);
            args.push("--file-lines".into());
            args.push(file_line_arg.into());
        }
        args.push(src_file.into());
        cmd.args(args);

        if let Some(current_dir) = current_dir_opt {
            cmd.current_dir(current_dir);
        }

        if self.args.quiet {
            self.run_suppress(cmd).expect("Couldn't rustfmt source file");
        } else {
            self.run_terminal(cmd).expect("Couldn't rustfmt source file");
        }
    }
}

fn get_cbmc_out(results_filename: PathBuf) -> Value {
    let results_file = fs::File::open(results_filename).unwrap();
    let reader = BufReader::new(results_file);
    let cbmc_out: Value = serde_json::from_reader(reader).unwrap();
    cbmc_out
}

fn handle_cbmc_out(cbmc_out: &Value) -> Vec<u8> {
    let mut det_vals: Vec<u8> = Vec::new();
    for general_msg in cbmc_out.as_array().unwrap() {
        let result_msg = &general_msg["result"];
        if !result_msg.is_null() {
            for result_val in result_msg.as_array().unwrap() {
                let mut det_vals_for_result = handle_result(result_val);
                det_vals.append(&mut det_vals_for_result);
            }
        }
    }
    det_vals
}

fn handle_result(result_val: &Value) -> Vec<u8> {
    let mut det_vals: Vec<u8> = Vec::new();
    let desc = result_val["description"].as_str().unwrap();

    if desc.contains("assertion failed") {
        for trace_val in result_val["trace"].as_array().unwrap() {
            let mut det_vals_for_trace = handle_trace(trace_val);
            det_vals.append(&mut det_vals_for_trace);
        }
    }

    det_vals
}

fn handle_trace(trace_val: &Value) -> Vec<u8> {
    let mut det_vals: Vec<u8> = Vec::new();
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
        let file_line_len = file_line.len();
        let file_line_no_quotes = &file_line[1..file_line_len - 1];
        let det_val_u8 = file_line_no_quotes.parse().unwrap();
        det_vals.push(det_val_u8);
    }

    det_vals
}
