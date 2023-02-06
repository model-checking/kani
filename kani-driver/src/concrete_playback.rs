// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module for parsing concrete values from CBMC output traces,
//! generating concrete playback unit tests, and adding them to the user's source code.

use crate::args::ConcretePlaybackMode;
use crate::call_cbmc::VerificationResult;
use crate::session::KaniSession;
use anyhow::{Context, Result};
use concrete_vals_extractor::{extract_harness_values, ConcreteVal};
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
        verification_result: &mut VerificationResult,
    ) -> Result<()> {
        let playback_mode = match self.args.concrete_playback {
            Some(playback_mode) => playback_mode,
            None => return Ok(()),
        };

        if let Some(result_items) = &verification_result.results {
            match extract_harness_values(result_items) {
                None => println!(
                    "WARNING: Kani could not produce a concrete playback for `{}` because there \
                    were no failing panic checks.",
                    harness.pretty_name
                ),
                Some(concrete_vals) => {
                    let concrete_playback = format_unit_test(&harness.pretty_name, &concrete_vals);
                    match playback_mode {
                        ConcretePlaybackMode::Print => {
                            println!(
                                "Concrete playback unit test for `{}`:\n```\n{}\n```",
                                &harness.pretty_name, &concrete_playback.unit_test_str
                            );
                            println!(
                                "INFO: To automatically add the concrete playback unit test `{}` to the \
                        src code, run Kani with `--concrete-playback=inplace`.",
                                &concrete_playback.unit_test_name
                            );
                        }
                        ConcretePlaybackMode::InPlace => {
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
                    verification_result.generated_concrete_test = true;
                }
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
            .with_context(|| format!("Couldn't create tmp source code file `{tmp_src_path}`"))?;
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
                "Couldn't convert source code file name `{src_file_name_as_osstr:?}` from OsStr to str"
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
    use crate::cbmc_output_parser::{CheckStatus, Property, TraceItem};

    #[derive(Hash)]
    pub struct ConcreteVal {
        pub byte_arr: Vec<u8>,
        pub interp_val: String,
    }

    /// Extract a set of concrete values that trigger one assertion failure.
    /// This will return None if the failure is not related to a user assertion.
    pub fn extract_harness_values(result_items: &[Property]) -> Option<Vec<ConcreteVal>> {
        let mut failures = result_items.iter().filter(|prop| {
            (prop.property_class() == "assertion" && prop.status == CheckStatus::Failure)
                || (prop.property_class() == "cover" && prop.status == CheckStatus::Satisfied)
        });

        // Process the first assertion failure.
        let first_failure = failures.next();
        if let Some(property) = first_failure {
            // Extract values for the first assertion that has failed.
            let trace = property
                .trace
                .as_ref()
                .expect(&format!("Missing trace for {}", property.property_name()));
            let concrete_vals = trace.iter().filter_map(&extract_from_trace_item).collect();

            // Print warnings for all the other failures that were not handled in case they expected
            // even future checks to be extracted.
            for unhandled in failures {
                println!(
                    "WARNING: Unable to extract concrete values from multiple failing assertions. Skipping property `{}` with description `{}`.",
                    unhandled.property_name(),
                    unhandled.description,
                );
            }
            Some(concrete_vals)
        } else {
            None
        }
    }

    /// Extracts individual bytes returned by kani::any() calls.
    fn extract_from_trace_item(trace_item: &TraceItem) -> Option<ConcreteVal> {
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
                    assert_eq!(
                        declared_width, actual_width,
                        "Declared width of {declared_width} doesn't equal actual width of {actual_width}"
                    );
                    let mut next_num: Vec<u8> = Vec::new();

                    // Reverse because of endianess of CBMC trace.
                    for i in (0..declared_width).step_by(8).rev() {
                        let str_chunk = &bit_concrete_val[i..i + 8];
                        let str_chunk_len = str_chunk.len();
                        assert_eq!(
                            str_chunk_len, 8,
                            "Tried to read a chunk of 8 bits of actually read {str_chunk_len} bits"
                        );
                        let next_byte = u8::from_str_radix(str_chunk, 2).expect(&format!(
                            "Couldn't convert the string chunk `{str_chunk}` to u8"
                        ));
                        next_num.push(next_byte);
                    }

                    return Some(ConcreteVal {
                        byte_arr: next_num,
                        interp_val: interp_concrete_val.to_string(),
                    });
                }
            }
        }
        None
    }
}

const SPACES_4: &str = "    ";
const SPACES_8: &str = "        ";

/// Format a unit test for a number of concrete values.
fn format_unit_test(harness_name: &str, concrete_vals: &[ConcreteVal]) -> UnitTest {
    // Hash the concrete values along with the proof harness name.
    let mut hasher = DefaultHasher::new();
    harness_name.hash(&mut hasher);
    concrete_vals.hash(&mut hasher);
    let hash = hasher.finish();
    let func_name = format!("kani_concrete_playback_{harness_name}_{hash}");

    let func_before_concrete_vals = [
        "#[test]".to_string(),
        format!("fn {func_name}() {{"),
        format!("{SPACES_4}let concrete_vals: Vec<Vec<u8>> = vec!["),
    ]
    .into_iter();
    let formatted_concrete_vals = format_concrete_vals(concrete_vals);
    let func_after_concrete_vals = [
        format!("{SPACES_4}];"),
        format!("{SPACES_4}kani::concrete_playback_run(concrete_vals, {harness_name});"),
        "}".to_string(),
    ]
    .into_iter();

    let full_func: Vec<_> = func_before_concrete_vals
        .chain(formatted_concrete_vals)
        .chain(func_after_concrete_vals)
        .collect();

    let full_func_code: String = full_func.join("\n");
    UnitTest { unit_test_str: full_func_code, unit_test_name: func_name }
}

/// Format an initializer expression for a number of concrete values.
fn format_concrete_vals(concrete_vals: &[ConcreteVal]) -> impl Iterator<Item = String> + '_ {
    /*
    Given a number of byte vectors, format them as:
    // interp_concrete_val_1
    vec![concrete_val_1],
    // interp_concrete_val_2
    vec![concrete_val_2], ...
    */
    concrete_vals.iter().flat_map(|concrete_val| {
        [
            format!("{SPACES_8}// {}", concrete_val.interp_val),
            format!("{SPACES_8}vec!{:?},", concrete_val.byte_arr),
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::concrete_vals_extractor::*;
    use super::*;
    use crate::cbmc_output_parser::{
        CheckStatus, Property, PropertyId, SourceLocation, TraceData, TraceItem, TraceValue,
    };

    #[test]
    fn format_zero_concrete_vals() {
        let concrete_vals: [ConcreteVal; 0] = [];
        let actual: Vec<_> = format_concrete_vals(&concrete_vals).collect();
        let expected: Vec<String> = Vec::new();
        assert_eq!(actual, expected);
    }

    #[test]
    fn format_two_concrete_vals() {
        let concrete_vals = [
            ConcreteVal { byte_arr: vec![0, 0], interp_val: "0".to_string() },
            ConcreteVal { byte_arr: vec![0, 0, 0, 0, 0, 0, 0, 0], interp_val: "0l".to_string() },
        ];
        let actual: Vec<_> = format_concrete_vals(&concrete_vals).collect();
        let expected = vec![
            format!("{SPACES_8}// 0"),
            format!("{SPACES_8}vec![0, 0],"),
            format!("{SPACES_8}// 0l"),
            format!("{SPACES_8}vec![0, 0, 0, 0, 0, 0, 0, 0],"),
        ];
        assert_eq!(actual, expected);
    }

    struct SplitUnitTestName {
        before_hash: String,
        hash: String,
    }

    /// Unit test names are formatted as "kani_concrete_playback_{harness_name}_{hash}".
    /// This function splits the name into "kani_concrete_playback_{harness_name}" and "{hash}".
    fn split_unit_test_name(unit_test_name: &str) -> SplitUnitTestName {
        let underscore_locs: Vec<_> = unit_test_name.match_indices('_').collect();
        let last_underscore_idx = underscore_locs[underscore_locs.len() - 1].0;
        SplitUnitTestName {
            before_hash: unit_test_name[..last_underscore_idx].to_string(),
            hash: unit_test_name[last_underscore_idx + 1..].to_string(),
        }
    }

    /// Since hashes can not be relied on in tests, this compares all parts of a unit test except the hash.
    #[test]
    fn format_unit_test_full_func() {
        let harness_name = "test_proof_harness";
        let concrete_vals = [ConcreteVal { byte_arr: vec![0, 0], interp_val: "0".to_string() }];
        let unit_test = format_unit_test(harness_name, &concrete_vals);
        let full_func: Vec<&str> = unit_test.unit_test_str.split('\n').collect();
        let split_unit_test_name = split_unit_test_name(&unit_test.unit_test_name);
        let expected_after_func_name = vec![
            format!("{SPACES_4}let concrete_vals: Vec<Vec<u8>> = vec!["),
            format!("{SPACES_8}// 0"),
            format!("{SPACES_8}vec![0, 0],"),
            format!("{SPACES_4}];"),
            format!("{SPACES_4}kani::concrete_playback_run(concrete_vals, {harness_name});"),
            "}".to_string(),
        ];

        assert_eq!(full_func[0], "#[test]");
        assert_eq!(
            split_unit_test_name.before_hash,
            format!("kani_concrete_playback_{harness_name}")
        );
        assert_eq!(full_func[1], format!("fn {}() {{", unit_test.unit_test_name));
        assert_eq!(full_func[2..], expected_after_func_name);
    }

    /// Generates a unit test and returns its hash.
    fn extract_hash_from_unit_test(harness_name: &str, concrete_vals: &[ConcreteVal]) -> String {
        let unit_test = format_unit_test(harness_name, concrete_vals);
        split_unit_test_name(&unit_test.unit_test_name).hash
    }

    /// Two hashes should not be the same if either the harness_name or the concrete_vals changes.
    #[test]
    fn check_hashes_are_unique() {
        let harness_name_1 = "test_proof_harness1";
        let harness_name_2 = "test_proof_harness2";
        let concrete_vals_1 = [ConcreteVal { byte_arr: vec![0, 0], interp_val: "0".to_string() }];
        let concrete_vals_2 = [ConcreteVal { byte_arr: vec![1, 0], interp_val: "0".to_string() }];
        let concrete_vals_3 = [ConcreteVal { byte_arr: vec![0, 0], interp_val: "1".to_string() }];

        let hash_base = extract_hash_from_unit_test(harness_name_1, &concrete_vals_1);
        let hash_diff_harness_name = extract_hash_from_unit_test(harness_name_2, &concrete_vals_1);
        let hash_diff_concrete_byte = extract_hash_from_unit_test(harness_name_1, &concrete_vals_2);
        let hash_diff_interp_val = extract_hash_from_unit_test(harness_name_1, &concrete_vals_3);

        assert_ne!(hash_base, hash_diff_harness_name);
        assert_ne!(hash_base, hash_diff_concrete_byte);
        assert_ne!(hash_base, hash_diff_interp_val);
    }

    #[test]
    fn check_concrete_vals_extractor() {
        let processed_items = [Property {
            description: "".to_string(),
            property_id: PropertyId {
                fn_name: Some("".to_string()),
                class: "assertion".to_string(),
                id: 1,
            },
            status: CheckStatus::Failure,
            reach: None,
            source_location: SourceLocation {
                column: None,
                file: None,
                function: None,
                line: None,
            },
            trace: Some(vec![TraceItem {
                thread: 0,
                step_type: "assignment".to_string(),
                hidden: false,
                lhs: Some("goto_symex$$return_value".to_string()),
                source_location: Some(SourceLocation {
                    column: None,
                    file: None,
                    function: Some("kani::any_raw_internal::<u8>".to_string()),
                    line: None,
                }),
                value: Some(TraceValue {
                    name: "".to_string(),
                    binary: Some("0000001100000001".to_string()),
                    data: Some(TraceData::NonBool("385".to_string())),
                    width: Some(16),
                }),
            }]),
        }];
        let concrete_vals = extract_harness_values(&processed_items).unwrap();
        let concrete_val = &concrete_vals[0];

        assert_eq!(concrete_val.byte_arr, vec![1, 3]);
        assert_eq!(concrete_val.interp_val, "385");
    }
}
