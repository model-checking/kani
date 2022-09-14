// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module for parsing concrete values from CBMC output traces,
//! generating concrete playback unit tests, and adding them to the user's source code.

use crate::args::ConcretePlaybackMode;
use crate::call_cbmc::VerificationStatus;
use crate::cbmc_output_parser::VerificationOutput;
use crate::harness_runner::HarnessResults;
use crate::session::KaniSession;
use anyhow::{ensure, Context, Result};
use concrete_vals_extractor::{extract_from_processed_items, ConcreteVal};
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
        harness: &HarnessMetadata,
        verification_output: &VerificationOutput,
    ) -> Result<()> {
        let playback_mode = match &self.args.concrete_playback {
            Some(playback_mode) => playback_mode,
            None => return Ok(()),
        };

        ensure!(
            self.args.output_format != crate::args::OutputFormat::Old,
            "The Kani argument `--output-format old` is not supported with the concrete playback feature."
        );

        if verification_output.status == VerificationStatus::Success {
            return Ok(());
        }

        if let Some(processed_items) = &verification_output.processed_items {
            let concrete_vals = extract_from_processed_items(processed_items).expect(
                "Something went wrong when trying to get concrete values from the CBMC output",
            );
            let concrete_playback = format_unit_test(&harness.mangled_name, &concrete_vals);

            if *playback_mode == ConcretePlaybackMode::Print {
                ensure!(
                    !self.args.quiet,
                    "With `--quiet` mode enabled, `--concrete-playback=print` mode can not print test cases."
                );
                println!(
                    "Concrete playback unit test for `{}`:\n```\n{}\n```",
                    &harness.mangled_name, &concrete_playback.unit_test_str
                );
                println!(
                    "INFO: To automatically add the concrete playback unit test `{}` to the src code, run Kani with `--concrete-playback=InPlace`.",
                    &concrete_playback.unit_test_name
                );
            }

            if *playback_mode == ConcretePlaybackMode::InPlace {
                if !self.args.quiet {
                    println!(
                        "INFO: Now modifying the source code to include the concrete playback unit test `{}`.",
                        &concrete_playback.unit_test_name
                    );
                }
                self.modify_src_code(
                    &harness.original_file,
                    harness.original_end_line,
                    &concrete_playback,
                )
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

    /// Helper function to inform the user that they tried to generate concrete playback unit tests when there were no failing harnesses.
    pub(crate) fn inform_if_no_failed(&self, results: &HarnessResults) {
        if self.args.concrete_playback.is_some() && !self.args.quiet && results.failures.is_empty()
        {
            println!(
                "INFO: The concrete playback feature never generated unit tests because there were no failing harnesses."
            )
        }
    }
}

/// Generate a unit test from a list of concrete values.
fn format_unit_test(harness_name: &str, concrete_vals: &[ConcreteVal]) -> UnitTest {
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
/// Note: we extract items that roughly look like the following:
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
    use crate::cbmc_output_parser::{
        extract_property_class, CheckStatus, ParserItem, Property, TraceItem,
    };
    use anyhow::{bail, ensure, Context, Result};

    pub struct ConcreteVal {
        pub byte_arr: Vec<u8>,
        pub interp_val: String,
    }

    /// The first-level extractor. Traverses processed items to find properties.
    pub fn extract_from_processed_items(
        processed_items: &[ParserItem],
    ) -> Result<Vec<ConcreteVal>> {
        let mut concrete_vals: Vec<ConcreteVal> = Vec::new();
        let mut extracted_assert_fail = false;
        let result_item = extract_result_from_processed_items(processed_items)?;
        for property in result_item {
            // Even after extracting an assert fail, we continue to call extract on more properties to provide
            // better diagnostics to the user in case they expected even future checks to be extracted.
            let old_extracted_assert_fail = extracted_assert_fail;
            let new_concrete_vals = extract_from_property(property, &mut extracted_assert_fail)?;
            if !old_extracted_assert_fail && extracted_assert_fail {
                concrete_vals = new_concrete_vals;
            }
        }
        Ok(concrete_vals)
    }

    /// Extracts the result item from all the processed items. No result item means that there is an error.
    fn extract_result_from_processed_items(processed_items: &[ParserItem]) -> Result<&[Property]> {
        for processed_item in processed_items {
            if let ParserItem::Result { result } = processed_item {
                return Ok(result);
            }
        }
        bail!("No result item found in processed items.")
    }

    /// The second-level extractor. Traverses properties to find trace items.
    fn extract_from_property(
        property: &Property,
        extracted_assert_fail: &mut bool,
    ) -> Result<Vec<ConcreteVal>> {
        let mut concrete_vals: Vec<ConcreteVal> = Vec::new();
        let property_class =
            extract_property_class(property).context("Incorrectly formatted property class.")?;
        let property_is_assert = property_class == "assertion";
        let status_is_failure = property.status == CheckStatus::Failure;

        if property_is_assert && status_is_failure {
            if *extracted_assert_fail {
                println!(
                    "WARNING: Unable to extract concrete values from multiple failing assertions. Skipping property `{}` with description `{}`.",
                    property.property, property.description,
                );
            } else {
                *extracted_assert_fail = true;
                println!(
                    "INFO: Parsing concrete values from property `{}` with description `{}`.",
                    property.property, property.description,
                );
                if let Some(trace) = &property.trace {
                    for trace_item in trace {
                        let concrete_val_opt = extract_from_trace_item(trace_item)
                            .context("Failure in trace assignment expression:")?;
                        if let Some(concrete_val) = concrete_val_opt {
                            concrete_vals.push(concrete_val);
                        }
                    }
                }
            }
        } else if !property_is_assert && status_is_failure {
            println!(
                "WARNING: Unable to extract concrete values from failing non-assertion checks. Skipping property `{}` with description `{}`.",
                property.property, property.description,
            );
        }
        Ok(concrete_vals)
    }

    /// The third-level extractor. Extracts individual bytes from kani::any calls.
    fn extract_from_trace_item(trace_item: &TraceItem) -> Result<Option<ConcreteVal>> {
        if let (Some(lhs), Some(source_location), Some(value)) =
            (&trace_item.lhs, &trace_item.source_location, &trace_item.value)
        {
            if let (
                Some(func),
                Some(width_u64),
                Some(bit_concrete_val),
                Some(interp_concrete_val),
            ) = (&source_location.function, value.width, &value.binary, &value.data)
            {
                if trace_item.step_type == "assignment"
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

                    return Ok(Some(ConcreteVal {
                        byte_arr: next_num,
                        interp_val: interp_concrete_val.to_string(),
                    }));
                }
            }
        }
        Ok(None)
    }
}
