// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module for parsing deterministic values from CBMC output traces,
//! generating executable trace unit tests, and adding them to the user's source code.

use crate::args::ConcretePlaybackMode;
use crate::session::KaniSession;
use anyhow::{Context, Result};
use kani_metadata::HarnessMetadata;
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

impl KaniSession {
    /// The main driver for generating executable traces and adding them to source code.
    pub fn gen_and_add_exe_trace(&self, output_filename: &Path, harness: &HarnessMetadata) {
        if self.args.concrete_playback.is_none() {
            return;
        }

        assert!(
            self.args.output_format != crate::args::OutputFormat::Old,
            "The Kani argument `--output-format old` is not supported with the executable trace feature."
        );

        let det_vals = parser::extract_det_vals(output_filename)
            .expect("Something went wrong when trying to get det vals from the CBMC output file");
        let exe_trace = format_unit_test(&harness.mangled_name, &det_vals);

        if let Some(playback_mode) = &self.args.concrete_playback && *playback_mode == ConcretePlaybackMode::JustPrint && !self.args.quiet {
            println!(
                "Executable trace for `{}`:\n```\n{}\n```",
                &harness.mangled_name, &exe_trace.unit_test_str
            );
            println!(
                "To automatically add the executable trace `{}` to the src code, run Kani with `--concrete-playback=InPlace`.",
                &exe_trace.unit_test_name
            );
        }

        if let Some(playback_mode) = &self.args.concrete_playback && *playback_mode == ConcretePlaybackMode::InPlace {
            if !self.args.quiet {
                println!(
                    "Now modifying the source code to include the unit test `{}`.",
                    &exe_trace.unit_test_name
                );
            }
            let proof_harness_end_line: usize = harness
                .original_end_line
                .parse()
                .expect(&format!("Invalid proof harness end line: {}", harness.original_end_line));
            self.modify_src_code(&harness.original_file, proof_harness_end_line, &exe_trace)
                .expect("Failed to modify source code");
        }
    }

    /// Add the exe trace to the user's source code, format it, and short circuit if code already present.
    fn modify_src_code(
        &self,
        src_path: &str,
        proof_harness_end_line: usize,
        exe_trace: &ExeTrace,
    ) -> Result<()> {
        let mut src_file = File::open(src_path)
            .with_context(|| format!("Couldn't open user's source code file `{src_path}`"))?;
        let mut src_as_str = String::new();
        src_file.read_to_string(&mut src_as_str).with_context(|| {
            format!("Couldn't read user's source code file `{src_path}` as a string")
        })?;

        // Short circuit if exe trace already in source code.
        if src_as_str.contains(&exe_trace.unit_test_name) {
            if !self.args.quiet {
                println!(
                    "Executable trace `{}/{}` already found in source code, so skipping modification.",
                    src_path, exe_trace.unit_test_name,
                );
            }
            return Ok(());
        }

        // Split the code into two different parts around the insertion point.
        let src_newline_matches: Vec<_> = src_as_str.match_indices('\n').collect();
        // If the proof harness ends on the last line of source code, there won't be a newline.
        let insertion_pt = if proof_harness_end_line == src_newline_matches.len() + 1 {
            src_as_str.len()
        } else {
            // Existing newline goes with 2nd src half. We also manually add newline before exe trace.
            src_newline_matches[proof_harness_end_line - 1].0
        };
        let src_before_exe_trace = &src_as_str[..insertion_pt];
        let src_after_exe_trace = &src_as_str[insertion_pt..];

        // Write new source lines to a tmp file, and then rename it to the actual user's source file.
        // Renames are usually automic, so we won't corrupt the user's source file during a crash.
        let tmp_src_path = src_path.to_string() + ".exe_trace_overwrite";
        let mut tmp_src_file = File::create(&tmp_src_path)
            .with_context(|| format!("Couldn't create tmp source code file `{}`", tmp_src_path))?;
        write!(
            tmp_src_file,
            "{}\n{}{}",
            src_before_exe_trace, exe_trace.unit_test_str, src_after_exe_trace
        )
        .with_context(|| {
            format!("Couldn't write new src str into tmp src file `{tmp_src_path}`")
        })?;
        fs::rename(&tmp_src_path, src_path).with_context(|| {
            format!("Couldn't rename tmp src file `{tmp_src_path}` to actual src file `{src_path}`")
        })?;

        // Run rustfmt on just the inserted lines.
        let source_path = Path::new(src_path);
        let parent_dir_as_path = source_path.parent().with_context(|| {
            format!("Expected source file `{}` to be in a directory", source_path.display())
        })?;
        let parent_dir_as_str = parent_dir_as_path.to_str().with_context(|| {
            format!(
                "Couldn't convert source file parent directory `{}` from  str",
                parent_dir_as_path.display()
            )
        })?;
        let src_file_name_as_osstr = source_path.file_name().with_context(|| {
            format!("Couldn't get the file name from the source file `{}`", source_path.display())
        })?;
        let src_file_name_as_str = src_file_name_as_osstr.to_str().with_context(|| {
            format!(
                "Couldn't convert source code file name `{:?}` from OsStr to str",
                src_file_name_as_osstr
            )
        })?;

        let exe_trace_num_lines = exe_trace.unit_test_str.matches('\n').count() + 1;
        let unit_test_start_line = proof_harness_end_line + 1;
        let unit_test_end_line = unit_test_start_line + exe_trace_num_lines - 1;
        let file_line_ranges = vec![FileLineRange {
            file: src_file_name_as_str.to_string(),
            line_range: Some((unit_test_start_line, unit_test_end_line)),
        }];
        self.run_rustfmt(&file_line_ranges, Some(parent_dir_as_str))?;
        Ok(())
    }

    /// Run rustfmt on the given src file, and optionally on only the specific lines.
    fn run_rustfmt(
        &self,
        file_line_ranges: &[FileLineRange],
        current_dir_opt: Option<&str>,
    ) -> Result<()> {
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
            self.run_suppress(cmd).context("Failed to rustfmt modified source code.")?;
        } else {
            self.run_terminal(cmd).context("Failed to rustfmt modified source code")?;
        }
        Ok(())
    }
}

/// Generate a unit test from a list of det vals.
fn format_unit_test(harness_name: &str, det_vals: &[parser::DetVal]) -> ExeTrace {
    /*
    Given a number of byte vectors, format them as:
    // interp_det_val_1
    vec![det_val_1],
    // interp_det_val_2
    vec![det_val_2], ...
    */
    let vec_whitespace = " ".repeat(8);
    let vecs_as_str = det_vals
        .iter()
        .map(|det_val| {
            format!(
                "{vec_whitespace}// {}\n{vec_whitespace}vec!{:?}",
                det_val.interp_val, det_val.byte_arr
            )
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
    kani::exe_trace_run(det_vals, {harness_name});
}}"
    );

    ExeTrace { unit_test_str: exe_trace, unit_test_name: exe_trace_func_name }
}

struct FileLineRange {
    file: String,
    line_range: Option<(usize, usize)>,
}

struct ExeTrace {
    unit_test_str: String,
    unit_test_name: String,
}

/// Read the CBMC output, parse it as a JSON object, and extract the deterministic values.
/// Note: the CBMC output should roughly look like this for parsing to work properly:
/// ```json
/// ...
/// { "result": [
///     ...,
///     { "description": "assertion failed: x", "status": "FAILURE", "trace": [
///         ...,
///         { "assignmentType": "variable", "lhs": "goto_symex$$return_value...",
///           "sourceLocation": { "function": "kani::any_raw_internal::<u8, 1_usize>" },
///           "stepType": "assignment", "value": { "binary": "00000001", "data": "101", "width": 8 } }
///         ..., ] }
///     ..., ] }
/// ```
/// TODO: this parser should be overhauled once the new Rust CBMC output parser is merged in.
/// Link to the new Rust parser PR: <https://github.com/model-checking/kani/pull/1433>.
/// Link to the issue: <https://github.com/model-checking/kani/issues/1477>.
mod parser {
    use anyhow::{ensure, Context, Result};
    use serde_json::Value;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    pub struct DetVal {
        pub byte_arr: Vec<u8>,
        pub interp_val: String,
    }

    /// Extract deterministic values from a failing harness.
    pub fn extract_det_vals(output_filename: &Path) -> Result<Vec<DetVal>> {
        let cbmc_out = read_cbmc_out(output_filename).with_context(|| {
            format!("Invalid CBMC output trace file: {}", output_filename.display())
        })?;
        let det_vals = parse_cbmc_out(&cbmc_out)
            .context("Failed to parse the CBMC output trace JSON to get det vals")?;
        Ok(det_vals)
    }

    /// Read in the CBMC results file and deserialize it to a JSON object.
    fn read_cbmc_out(results_filename: &Path) -> Result<Value> {
        let results_file =
            File::open(results_filename).context("Failed to open CBMC output trace file")?;
        let reader = BufReader::new(results_file);
        let json_val = serde_json::from_reader(reader)
            .context("Could not convert CBMC output trace file into valid JSON")?;
        Ok(json_val)
    }

    /// The first-level CBMC output parser. This extracts the result message.
    fn parse_cbmc_out(cbmc_out: &Value) -> Result<Vec<DetVal>> {
        let mut det_vals: Vec<DetVal> = Vec::new();
        let cbmc_out_arr = cbmc_out
            .as_array()
            .with_context(|| format!("Expected this CBMC output to be an array: {}", cbmc_out))?;
        let mut have_parsed_assert_fail = false;
        for general_msg in cbmc_out_arr {
            let result_msg = &general_msg["result"];
            if !result_msg.is_null() {
                let result_arr = result_msg.as_array().with_context(|| {
                    format!("Expected this CBMC result object to be an array: {}", result_msg)
                })?;
                for result_val in result_arr {
                    parse_result(result_val, &mut det_vals, &mut have_parsed_assert_fail)?;
                }
            }
        }
        Ok(det_vals)
    }

    /// The second-level CBMC output parser. This extracts the trace entries of failing assertions.
    fn parse_result(
        result_val: &Value,
        det_vals: &mut Vec<DetVal>,
        have_parsed_assert_fail: &mut bool,
    ) -> Result<()> {
        let desc = result_val["description"].to_string();
        let prop = result_val["property"].to_string();
        let status = result_val["status"].to_string();
        let prop_is_assert = prop.contains("assertion");
        let status_is_failure = status == "\"FAILURE\"";

        if prop_is_assert && status_is_failure {
            if *have_parsed_assert_fail {
                println!(
                    "WARNING: Unable to parse deterministic values from multiple failing assertions. Skipping property `{prop}` with description `{desc}`."
                );
            } else {
                *have_parsed_assert_fail = true;
                println!(
                    "INFO: Parsing deterministic values from property `{prop}` with description `{desc}`."
                );
                let trace_arr = result_val["trace"].as_array().with_context(|| {
                    format!(
                        "Expected this CBMC result trace to be an array: {}",
                        result_val["trace"]
                    )
                })?;
                for trace_entry in trace_arr {
                    parse_trace_entry(trace_entry, det_vals)
                        .context("Failure in trace assignment expression:")?;
                }
            }
        } else if !prop_is_assert && status_is_failure {
            println!(
                "WARNING: Unable to parse deterministic values from failing non-assertion checks. Skipping property `{prop}` with description `{desc}`."
            );
        }
        Ok(())
    }

    /// The third-level CBMC output parser. This extracts individual bytes from kani::any_raw calls.
    fn parse_trace_entry(trace_entry: &Value, det_vals: &mut Vec<DetVal>) -> Result<()> {
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
        ) {
            if step_type == "assignment"
                && lhs.starts_with("goto_symex$$return_value")
                && func.starts_with("kani::any_raw_internal")
            {
                let declared_width = width_u64 as usize;
                let actual_width = bit_det_val.len();
                ensure!(
                    declared_width == actual_width,
                    format!(
                        "Declared width of {declared_width} doesn't equal actual width of {actual_width}"
                    )
                );
                let mut next_num: Vec<u8> = Vec::new();

                // Reverse because of endianess of CBMC trace.
                for i in (0..declared_width).step_by(8).rev() {
                    let str_chunk = &bit_det_val[i..i + 8];
                    let str_chunk_len = str_chunk.len();
                    ensure!(
                        str_chunk_len == 8,
                        format!(
                            "Tried to read a chunk of 8 bits of actually read {str_chunk_len} bits"
                        )
                    );
                    let next_byte = u8::from_str_radix(str_chunk, 2).with_context(|| {
                        format!("Couldn't convert the string chunk `{str_chunk}` to u8")
                    })?;
                    next_num.push(next_byte);
                }

                det_vals
                    .push(DetVal { byte_arr: next_num, interp_val: interp_det_val.to_string() });
            }
        }
        Ok(())
    }
}
