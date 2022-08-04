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
            let (exe_trace, exe_trace_func_name) = format_unit_test(
                &harness.mangled_name,
                det_vals.as_slice(),
                interp_det_vals.as_slice(),
            );

            println!("Executable trace:");
            println!("```");
            println!("{}", exe_trace);
            println!("```");

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
        let file_line_ranges = vec![FileLineRange {
            file: src_file.to_string(),
            line_range: Some((start_line, end_line)),
        }];
        self.run_rustfmt(file_line_ranges.as_slice(), Some(parent_dir));
    }

    /// Run rustfmt on the given src file, and optionally on only the specific lines.
    fn run_rustfmt(&self, file_line_ranges: &[FileLineRange], current_dir_opt: Option<&str>) {
        let mut cmd = Command::new("rustfmt");
        let mut args: Vec<OsString> = Vec::new();

        // Deal with file line ranges.
        let mut line_range_dicts: Vec<String> = Vec::new();
        for file_line_range in file_line_ranges {
            if let Some((start_line, end_line)) = file_line_range.line_range {
                let src_file = &file_line_range.file;
                let line_range_dict =
                    format!("{{\"file\":\"{src_file}\",\"range\":[{start_line},{end_line}]}}");
                line_range_dicts.push(line_range_dict);
            }
        }
        if !line_range_dicts.is_empty() {
            // `--file-lines` arg is currently unstable.
            args.push("--unstable-features".into());
            args.push("--file-lines".into());
            let line_range_dicts_combined = format!("[{}]", line_range_dicts.join(","));
            args.push(line_range_dicts_combined.into());
        }

        for file_line_range in file_line_ranges {
            args.push((&file_line_range.file).into());
        }

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
        .map(|(det_val, interp_det_val)| {
            format!("        // {interp_det_val}\n        vec!{:?}", det_val)
        })
        .collect::<Vec<String>>()
        .join(",\n");

    // Hash the generated det val string along with the proof harness name.
    let mut hasher = DefaultHasher::new();
    harness_name.hash(&mut hasher);
    vecs_as_str.hash(&mut hasher);
    let hash = hasher.finish();

    let exe_trace_func_name = format!("kani_exe_trace_{harness_name}_{hash}");
    #[rustfmt::skip]
    let exe_trace = format!(
"#[test]
fn {exe_trace_func_name}() {{
    let det_vals: Vec<Vec<u8>> = vec![
{vecs_as_str}
    ];
    kani::exe_trace_init(det_vals);
    {harness_name}();
}}"
    );

    (exe_trace, exe_trace_func_name)
}
struct FileLineRange {
    file: String,
    line_range: Option<(usize, usize)>,
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
        parse_cbmc_out(&cbmc_out)
    }

    /// Read in the CBMC results file and deserialize it to a JSON object.
    fn get_cbmc_out(results_filename: &Path) -> Value {
        let results_file = File::open(results_filename).unwrap();
        let reader = BufReader::new(results_file);
        serde_json::from_reader(reader).unwrap()
    }

    /// The first-level CBMC output parser. This extracts the result message.
    fn parse_cbmc_out(cbmc_out: &Value) -> (Vec<Vec<u8>>, Vec<String>) {
        let mut det_vals: Vec<Vec<u8>> = Vec::new();
        let mut interp_det_vals: Vec<String> = Vec::new();
        for general_msg in cbmc_out.as_array().unwrap() {
            let result_msg = &general_msg["result"];
            if !result_msg.is_null() {
                for result_val in result_msg.as_array().unwrap() {
                    parse_result(result_val, &mut det_vals, &mut interp_det_vals);
                }
            }
        }
        (det_vals, interp_det_vals)
    }

    /// The second-level CBMC output parser. This extracts the trace entries of failing assertions.
    fn parse_result(
        result_val: &Value,
        det_vals: &mut Vec<Vec<u8>>,
        interp_det_vals: &mut Vec<String>,
    ) {
        let desc = result_val["description"].to_string();
        let status = result_val["status"].to_string();

        if desc.contains("assertion failed") && status == "\"FAILURE\"" {
            for trace_entry in result_val["trace"].as_array().unwrap() {
                parse_trace_entry(trace_entry, det_vals, interp_det_vals);
            }
        }
    }

    /// The third-level CBMC output parser. This extracts individual bytes from kani::any_raw calls.
    fn parse_trace_entry(
        trace_entry: &Value,
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
            trace_entry["stepType"].as_str(),
            trace_entry["lhs"].as_str(),
            trace_entry["sourceLocation"]["function"].as_str(),
            trace_entry["value"]["binary"].as_str(),
            trace_entry["value"]["data"].as_str(),
            trace_entry["value"]["width"].as_u64(),
        ) && step_type == "assignment" && lhs == "var_0" && func.starts_with("kani::any_raw_internal") {
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
