// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::session::KaniSession;
use kani_metadata::HarnessMetadata;
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;

/// Module for parsing deterministic values from CBMC output traces,
/// generating executable traces, and adding them to the user's source code.

impl KaniSession {
    /// The main driver for generating executable traces and adding them to source code.
    pub fn exe_trace_main(&self, specialized_obj: &Path, harness: &HarnessMetadata) {
        if self.args.gen_exe_trace {
            let (det_vals, interp_det_vals) = parser::get_det_vals(specialized_obj);
            let (exe_trace, exe_trace_func_name) =
                format_unit_test(&harness.mangled_name, &det_vals[..], &interp_det_vals[..]);

            println!("Executable trace:\n");
            println!("{}", exe_trace);

            if self.args.add_exe_trace_to_src {
                if !self.args.quiet {
                    println!("Now modifying the source code to include the executable trace.");
                }
                let proof_harness_end_line: usize = harness
                    .original_end_line
                    .parse()
                    .expect("Couldn't convert proof harness line from str to usize");
                self.modify_src_code(
                    &harness.original_file,
                    proof_harness_end_line,
                    &exe_trace,
                    &exe_trace_func_name,
                );
            } else {
                println!(
                    "To automatically add this executable trace to the src code, run Kani with `--add-exe-trace-to-src`."
                );
            }
        }
    }

    /// Add the exe trace to the user's source code, format it, and short circuit if code already present.
    fn modify_src_code(
        &self,
        src_file_path: &str,
        proof_harness_end_line: usize,
        exe_trace: &str,
        exe_trace_func_name: &str,
    ) {
        // Prep the source code and exec trace func.
        let src_as_str =
            fs::read_to_string(src_file_path).expect("Couldn't access proof harness source file");
        let mut src_lines = src_as_str.split('\n').collect::<Vec<&str>>();
        let mut exe_trace_lines = exe_trace.split('\n').collect::<Vec<&str>>();

        // Short circuit if exe trace already in source code.
        if src_as_str.contains(exe_trace_func_name) {
            if !self.args.quiet {
                println!("Exe trace already found in source code, so skipping modification.");
            }
            return;
        }

        // Calc inserted indexes and line numbers into source code to rustfmt only those lines.
        // Indexes are into the vector (0-idx), lines are into src file (1-idx).
        let start_line = proof_harness_end_line + 1;
        let start_idx = start_line - 1;
        // If start_line=2, exe_trace.len()=3, then inserted code ends at line 4 (start + len - 1).
        let end_line = start_line + exe_trace_lines.len() - 1;

        // Splice the exec trace func into the proof harness source code.
        // start_idx is at max len(src_lines), so no panic here, even if proof harness is at the end of src file.
        let mut src_right = src_lines.split_off(start_idx);
        src_lines.append(&mut exe_trace_lines);
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

    /// Run rustfmt on the given src file, and optionally on only the specific lines.
    fn run_rustfmt(
        &self,
        src_file: &str,
        current_dir_opt: Option<&str>,
        start_line_opt: Option<usize>,
        end_line_opt: Option<usize>,
    ) {
        let mut cmd = Command::new("rustfmt");

        let mut args: Vec<OsString> = Vec::new();
        if let (Some(start_line), Some(end_line)) = (start_line_opt, end_line_opt) {
            // `--file-lines` arg is currently unstable.
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

/// Generate a unit test from a list of det vals.
fn format_unit_test(
    harness_name: &str,
    det_vals: &[Vec<u8>],
    interp_det_vals: &[String],
) -> (String, String) {
    /*
    Given a number of byte vectors, format them as:
    // interp_det_val_1
    vec![det_val_1],
    // interp_det_val_2
    vec![det_val_2], ...
    */
    let vecs_as_str = det_vals
        .iter()
        .zip(interp_det_vals.iter())
        .map(|(det_val, interp_det_val)| format!("// {}\nvec!{:?}", interp_det_val, det_val))
        .collect::<Vec<String>>()
        .join(",\n");

    // Hash the generated det val string along with the proof harness name.
    let mut hasher = DefaultHasher::new();
    harness_name.hash(&mut hasher);
    vecs_as_str.hash(&mut hasher);
    let hash = hasher.finish();

    let exe_trace_func_name = format!("kani_exe_trace_{}_{}", harness_name, hash);
    let exe_trace = format!(
        "
        #[test]
        fn {}() {{
            let det_vals: Vec<Vec<u8>> = vec![{}];
            kani::exe_trace_init(det_vals);
            {}();
        }}",
        exe_trace_func_name, vecs_as_str, harness_name
    );

    (exe_trace, exe_trace_func_name)
}

/// Read the CBMC output, parse it as a JSON object, and extract the deterministic values.
mod parser {
    use crate::util::append_path;
    use serde_json::Value;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    /// Extract deterministic values from a failing harness.
    pub fn get_det_vals(file: &Path) -> (Vec<Vec<u8>>, Vec<String>) {
        let output_filename = append_path(file, "cbmc_output");
        let cbmc_out = get_cbmc_out(&output_filename);
        handle_cbmc_out(&cbmc_out)
    }

    /// Read in the CBMC results file and deserialize it to a JSON object.
    fn get_cbmc_out(results_filename: &Path) -> Value {
        let results_file = File::open(results_filename).unwrap();
        let reader = BufReader::new(results_file);
        serde_json::from_reader(reader).unwrap()
    }

    /// The first-level JSON object parser. This extracts the result message.
    fn handle_cbmc_out(cbmc_out: &Value) -> (Vec<Vec<u8>>, Vec<String>) {
        let mut det_vals: Vec<Vec<u8>> = Vec::new();
        let mut interp_det_vals: Vec<String> = Vec::new();
        for general_msg in cbmc_out.as_array().unwrap() {
            let result_msg = &general_msg["result"];
            if !result_msg.is_null() {
                for result_val in result_msg.as_array().unwrap() {
                    handle_result(result_val, &mut det_vals, &mut interp_det_vals);
                }
            }
        }
        (det_vals, interp_det_vals)
    }

    /// The second-level JSON object parser. This extracts the traces of failing assertions.
    fn handle_result(
        result_val: &Value,
        det_vals: &mut Vec<Vec<u8>>,
        interp_det_vals: &mut Vec<String>,
    ) {
        let desc = result_val["description"].to_string();
        let status = result_val["status"].to_string();

        if desc.contains("assertion failed") && status == "\"FAILURE\"" {
            for trace_pt in result_val["trace"].as_array().unwrap() {
                handle_trace_pt(trace_pt, det_vals, interp_det_vals);
            }
        }
    }

    /// The third-level of JSON object parser. This extracts individual bytes from kani::any_raw calls.
    fn handle_trace_pt(
        trace_pt: &Value,
        det_vals: &mut Vec<Vec<u8>>,
        interp_det_vals: &mut Vec<String>,
    ) {
        if let (
            Some(step_type),
            Some(lhs),
            Some(func),
            Some(bit_det_val),
            Some(interp_det_val),
            Some(width_u64),
        ) = (
            trace_pt["stepType"].as_str(),
            trace_pt["lhs"].as_str(),
            trace_pt["sourceLocation"]["function"].as_str(),
            trace_pt["value"]["binary"].as_str(),
            trace_pt["value"]["data"].as_str(),
            trace_pt["value"]["width"].as_u64(),
        ) && step_type == "assignment" && lhs == "var_0" && func.starts_with("kani::any_raw") {
            // TODO: Change all these unwrap panics to send their errors up.
            let width = width_u64 as usize;
            assert_eq!(width, bit_det_val.len(), "Declared width wasn't same as width found in bit string");
            let mut next_num: Vec<u8> = Vec::new();

            // Reverse because of endianess of CBMC trace.
            for i in (0..width).step_by(8).rev() {
                let str_chunk = &bit_det_val[i..i+8];
                let next_byte = u8::from_str_radix(str_chunk, 2).unwrap();
                next_num.push(next_byte);
            }

            det_vals.push(next_num);
            interp_det_vals.push(interp_det_val.to_string());
        }
    }
}
