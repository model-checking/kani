// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MI&T

use std::ffi::OsString;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::KaniSession;
use crate::util::append_path;
use anyhow::Result;
use kani_metadata::HarnessMetadata;
use serde_json::Value;

impl KaniSession {
    pub fn exe_trace_main(&self, specialized_obj: &Path, harness: &HarnessMetadata) {
        if self.args.gen_exe_trace {
            let det_vals = self
                .get_det_vals(specialized_obj)
                .expect("Something went wrong in extracting determinstic values.");
            let unit_test = self.format_unit_test(&harness.mangled_name, &det_vals);

            println!("Executable trace:\n");
            println!("{}", unit_test);

            if self.args.add_exe_trace_to_src {
                if !self.args.quiet {
                    println!("Now modifying the source code to include the executable trace.");
                }
                let proof_harness_end_line: usize = harness
                    .original_end_line
                    .parse()
                    .expect("Couldn't convert proof harness line from str to usize");
                self.modify_src_code(&harness.original_file, proof_harness_end_line, &unit_test);
            } else {
                println!(
                    "To automatically add this executable trace to the src code, run Kani with `--add-exe-trace-to-src`."
                );
            }
        }
    }

    /// Extract deterministic values from a failing harness.
    pub fn get_det_vals(&self, file: &Path) -> Result<Vec<u8>> {
        let output_filename = append_path(file, "cbmc_output");
        let cbmc_out = get_cbmc_out(output_filename);
        let det_vals = handle_cbmc_out(&cbmc_out);
        Ok(det_vals)
    }

    pub fn format_unit_test(&self, harness_name: &str, det_vals: &Vec<u8>) -> String {
        format!(
            "
            #[test]
            fn kani_autogen_{}_exe_trace() {{
                kani::DET_VALS.with(|det_vals| {{
                    *det_vals.borrow_mut() = vec!{:?};
                }});
                {}();
            }}",
            harness_name, det_vals, harness_name
        )
    }

    pub fn modify_src_code(
        &self,
        src_file_path: &str,
        proof_harness_end_line: usize,
        exec_trace: &str,
    ) {
        // Prep the source code and exec trace func.
        let src_as_str =
            fs::read_to_string(src_file_path).expect("Couldn't access proof harness source file");
        let mut src_lines = src_as_str.split('\n').collect::<Vec<&str>>();
        let mut exec_trace_lines = exec_trace.split('\n').collect::<Vec<&str>>();

        // Calc inserted indexes and line numbers into source code to rustfmt only those lines.
        // Indexes are into the vector (0-idx), lines are into src file (1-idx).
        let start_line = proof_harness_end_line + 1;
        let start_idx = start_line - 1;
        // If start_line=2, exe_trace.len()=3, then inserted code ends at line 4 (start + len - 1).
        let end_line = start_line + exec_trace_lines.len() - 1;

        // Splice the exec trace func into the proof harness source code.
        // start_idx is at max len(src_lines), so no panic here, even if proof harness is at the end of src file.
        let mut src_right = src_lines.split_off(start_idx);
        src_lines.append(&mut exec_trace_lines);
        src_lines.append(&mut src_right);
        let new_src = src_lines.join("\n");
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
    // Det vals are popped off the Vec, so need to reverse.
    det_vals.reverse();
    det_vals
}

fn handle_result(result_val: &Value) -> Vec<u8> {
    let mut det_vals: Vec<u8> = Vec::new();
    let desc = result_val["description"].as_str().unwrap();
    let status = result_val["status"].as_str().unwrap();

    if desc.contains("assertion failed") && status == "FAILURE" {
        for trace_pt in result_val["trace"].as_array().unwrap() {
            let det_val_opt = handle_trace_pt(trace_pt);
            if let Some(det_val) = det_val_opt {
                det_vals.push(det_val);
            }
        }
    }

    det_vals
}

fn handle_trace_pt(trace_pt: &Value) -> Option<u8> {
    let step_type = &trace_pt["stepType"];
    if step_type != "assignment" {
        return None;
    }

    let lhs = trace_pt["lhs"].as_str().unwrap();
    if !lhs.starts_with("non_det_byte_arr") {
        return None;
    }

    let func = trace_pt["sourceLocation"]["function"].as_str().unwrap();
    if !func.starts_with("kani::any_raw") {
        return None;
    }

    let det_val_with_quotes = trace_pt["value"]["data"].to_string();
    let det_val_no_quotes = &det_val_with_quotes[1..det_val_with_quotes.len() - 1];
    let det_val_u8: u8 = det_val_no_quotes.parse().expect("Couldn't parse the trace byte as a u8");
    Some(det_val_u8)
}
