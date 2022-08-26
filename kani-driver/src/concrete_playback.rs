// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module for parsing concrete values from CBMC output traces,
//! generating concrete playback unit tests, and adding them to the user's source code.

use crate::args::ConcretePlaybackMode;
use crate::call_cbmc::VerificationStatus;
use crate::cbmc_output_parser::VerificationResult;
use crate::session::KaniSession;
use anyhow::{ensure, Context, Result};
use kani_metadata::HarnessMetadata;
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

impl KaniSession {
    /// The main driver for generating concrete playback unit tests and adding them to source code.
    pub fn gen_and_add_concrete_playback(
        &self,
        output_filename: &Path,
        harness: &HarnessMetadata,
        verification_result: &VerificationResult,
    ) -> Result<()> {
        if self.args.concrete_playback.is_none() {
            return Ok(());
        }

        ensure!(
            self.args.output_format != crate::args::OutputFormat::Old,
            "The Kani argument `--output-format old` is not supported with the concrete playback feature."
        );

        if verification_result.status == VerificationStatus::Success {
            if !self.args.quiet {
                println!(
                    "INFO: The concrete playback feature does not generate unit tests when verification succeeds."
                )
            }
            return Ok(());
        }

        if let Some(_processed_items) = &verification_result.processed_items {
            // TODO: rename parser to extractor
            let concrete_vals = concrete_vals_extractor::extract_concrete_vals(output_filename).expect(
                "Something went wrong when trying to get concrete values from the CBMC output file",
            );
            let concrete_playback = format_unit_test(&harness.mangled_name, &concrete_vals);

            if let Some(playback_mode) = &self.args.concrete_playback && *playback_mode == ConcretePlaybackMode::Print && !self.args.quiet {
                println!(
                    "Concrete playback unit test for `{}`:\n```\n{}\n```",
                    &harness.mangled_name, &concrete_playback.unit_test_str
                );
                println!(
                    "INFO: To automatically add the concrete playback unit test `{}` to the src code, run Kani with `--concrete-playback=InPlace`.",
                    &concrete_playback.unit_test_name
                );
            }

            if let Some(playback_mode) = &self.args.concrete_playback && *playback_mode == ConcretePlaybackMode::InPlace {
                if !self.args.quiet {
                    println!(
                        "INFO: Now modifying the source code to include the concrete playback unit test `{}`.",
                        &concrete_playback.unit_test_name
                    );
                }
                self.modify_src_code(&harness.original_file, harness.original_end_line, &concrete_playback)
                    .expect("Failed to modify source code");
            }
        }
        Ok(())
    }

    /// Add the unit test to the user's source code, format it, and short circuit if code already present.
    fn modify_src_code(
        &self,
        src_path: &str,
        proof_harness_end_line: usize,
        concrete_playback: &UnitTest,
    ) -> Result<()> {
        let mut src_file = File::open(src_path)
            .with_context(|| format!("Couldn't open user's source code file `{src_path}`"))?;
        let mut src_as_str = String::new();
        src_file.read_to_string(&mut src_as_str).with_context(|| {
            format!("Couldn't read user's source code file `{src_path}` as a string")
        })?;

        // Short circuit if unit test already in source code.
        if src_as_str.contains(&concrete_playback.unit_test_name) {
            if !self.args.quiet {
                println!(
                    "Concrete playback unit test `{}/{}` already found in source code, so skipping modification.",
                    src_path, concrete_playback.unit_test_name,
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
            // Existing newline goes with 2nd src half. We also manually add newline before unit test.
            src_newline_matches[proof_harness_end_line - 1].0
        };
        let src_before_concrete_playback = &src_as_str[..insertion_pt];
        let src_after_concrete_playback = &src_as_str[insertion_pt..];

        // Write new source lines to a tmp file, and then rename it to the actual user's source file.
        // Renames are usually automic, so we won't corrupt the user's source file during a crash.
        let tmp_src_path = src_path.to_string() + ".concrete_playback_overwrite";
        let mut tmp_src_file = File::create(&tmp_src_path)
            .with_context(|| format!("Couldn't create tmp source code file `{}`", tmp_src_path))?;
        write!(
            tmp_src_file,
            "{}\n{}{}",
            src_before_concrete_playback,
            concrete_playback.unit_test_str,
            src_after_concrete_playback
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

        let concrete_playback_num_lines = concrete_playback.unit_test_str.matches('\n').count() + 1;
        let unit_test_start_line = proof_harness_end_line + 1;
        let unit_test_end_line = unit_test_start_line + concrete_playback_num_lines - 1;
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

/// Generate a unit test from a list of concrete values.
fn format_unit_test(harness_name: &str, concrete_vals: &[concrete_vals_extractor::ConcreteVal]) -> UnitTest {
    /*
    Given a number of byte vectors, format them as:
    // interp_concrete_val_1
    vec![concrete_val_1],
    // interp_concrete_val_2
    vec![concrete_val_2], ...
    */
    let vec_whitespace = " ".repeat(8);
    let vecs_as_str = concrete_vals
        .iter()
        .map(|concrete_val| {
            format!(
                "{vec_whitespace}// {}\n{vec_whitespace}vec!{:?}",
                concrete_val.interp_val, concrete_val.byte_arr
            )
        })
        .collect::<Vec<String>>()
        .join(",\n");

    // Hash the generated det val string along with the proof harness name.
    let mut hasher = DefaultHasher::new();
    harness_name.hash(&mut hasher);
    vecs_as_str.hash(&mut hasher);
    let hash = hasher.finish();

    let concrete_playback_func_name = format!("kani_concrete_playback_{harness_name}_{hash}");
    #[rustfmt::skip]
    let concrete_playback = format!(
"#[test]
fn {concrete_playback_func_name}() {{
    let concrete_vals: Vec<Vec<u8>> = vec![
{vecs_as_str}
    ];
    kani::concrete_playback_run(concrete_vals, {harness_name});
}}"
    );

    UnitTest { unit_test_str: concrete_playback, unit_test_name: concrete_playback_func_name }
}

struct FileLineRange {
    file: String,
    line_range: Option<(usize, usize)>,
}

struct UnitTest {
    unit_test_str: String,
    unit_test_name: String,
}

/// Extract concrete values from the CBMC output processed items.
/// Note: we parse items that roughly look like the following:
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
mod concrete_vals_extractor {
    use anyhow::{ensure, Context, Result};
    use serde_json::Value;
    use std::path::Path;
    use crate::cbmc_output_parser::{ParserItem, Property};

    pub struct ConcreteVal {
        pub byte_arr: Vec<u8>,
        pub interp_val: String,
    }

    /// The first-level extractor. Traverses processed items for properties.
    pub fn extract_from_processed_items(processed_items: &[ParserItem]) -> Result<Vec<ConcreteVal>> {
        let mut concrete_vals: Vec<ConcreteVal> = Vec::new();
        let mut have_parsed_assert_fail = false;
        for processed_item in processed_items.iter() {
            if let ParserItem::Result { result } = processed_item {
                for property in result.iter() {
                    extract_from_property(property, &mut concrete_vals, &mut have_parsed_assert_fail);
                }
            }
        }
        Ok(concrete_vals)
    }

    /// The second-level extractor. Extracts det vals 
    pub fn extract_from_property(property: &Property, concrete_vals: &mut Vec<ConcreteVal>, have_parsed_assert_fail: &mut bool) {
            
    }

    /// The second-level CBMC output parser. This extracts the trace entries of failing assertions.
    fn parse_result(
        result_val: &Value,
        concrete_vals: &mut Vec<ConcreteVal>,
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
                    "WARNING: Unable to parse concrete values from multiple failing assertions. Skipping property `{prop}` with description `{desc}`."
                );
            } else {
                *have_parsed_assert_fail = true;
                println!(
                    "INFO: Parsing concrete values from property `{prop}` with description `{desc}`."
                );
                let trace_arr = result_val["trace"].as_array().with_context(|| {
                    format!(
                        "Expected this CBMC result trace to be an array: {}",
                        result_val["trace"]
                    )
                })?;
                for trace_entry in trace_arr {
                    parse_trace_entry(trace_entry, concrete_vals)
                        .context("Failure in trace assignment expression:")?;
                }
            }
        } else if !prop_is_assert && status_is_failure {
            println!(
                "WARNING: Unable to parse concrete values from failing non-assertion checks. Skipping property `{prop}` with description `{desc}`."
            );
        }
        Ok(())
    }

    /// The third-level CBMC output parser. This extracts individual bytes from kani::any_raw calls.
    fn parse_trace_entry(trace_entry: &Value, concrete_vals: &mut Vec<ConcreteVal>) -> Result<()> {
        if let (
            Some(step_type),
            Some(lhs),
            Some(func),
            Some(bit_concrete_val),
            Some(interp_concrete_val),
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
                let actual_width = bit_concrete_val.len();
                ensure!(
                    declared_width == actual_width,
                    format!(
                        "Declared width of {declared_width} doesn't equal actual width of {actual_width}"
                    )
                );
                let mut next_num: Vec<u8> = Vec::new();

                // Reverse because of endianess of CBMC trace.
                for i in (0..declared_width).step_by(8).rev() {
                    let str_chunk = &bit_concrete_val[i..i + 8];
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

                concrete_vals.push(ConcreteVal {
                    byte_arr: next_num,
                    interp_val: interp_concrete_val.to_string(),
                });
            }
        }
        Ok(())
    }
}
