// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module for parsing concrete values from CBMC output traces,
//! generating concrete playback unit tests, and adding them to the user's source code.

use crate::args::ConcretePlaybackMode;
use crate::call_cbmc::VerificationResult;
use crate::session::KaniSession;
use crate::util::tempfile::TempFile;
use anyhow::{Context, Result};
use concrete_vals_extractor::{extract_harness_values, ConcreteVal};
use kani_metadata::HarnessMetadata;
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
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

        if let Ok(result_items) = &verification_result.results {
            let harness_values: Vec<Vec<ConcreteVal>> = extract_harness_values(result_items);

            if harness_values.is_empty() {
                println!(
                    "WARNING: Kani could not produce a concrete playback for `{}` because there \
                    were no failing panic checks.",
                    harness.pretty_name
                )
            } else {
                for concrete_vals in harness_values.iter() {
                    let pretty_name = harness.get_harness_name_unqualified();
                    let generated_unit_test = format_unit_test(&pretty_name, &concrete_vals);
                    match playback_mode {
                        ConcretePlaybackMode::Print => {
                            println!(
                                "Concrete playback unit test for `{}`:\n```\n{}\n```",
                                &harness.pretty_name,
                                &generated_unit_test.code.join("\n")
                            );
                            println!(
                                "INFO: To automatically add the concrete playback unit test `{}` to the \
                        src code, run Kani with `--concrete-playback=inplace`.",
                                &generated_unit_test.name
                            );
                        }
                        ConcretePlaybackMode::InPlace => {
                            if !self.args.common_args.quiet {
                                println!(
                                    "INFO: Now modifying the source code to include the concrete playback unit test `{}`.",
                                    &generated_unit_test.name
                                );
                            }
                            self.modify_src_code(
                                &harness.original_file,
                                harness.original_end_line,
                                &generated_unit_test,
                            )
                            .expect(&format!(
                                "Failed to modify source code for the file `{}`",
                                &harness.original_file
                            ));
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
        unit_test: &UnitTest,
    ) -> Result<()> {
        let unit_test_already_in_src =
            self.add_test_inplace(src_path, proof_harness_end_line, unit_test)?;

        if unit_test_already_in_src {
            return Ok(());
        }

        // Run rustfmt on just the inserted lines.
        let concrete_playback_num_lines = unit_test.code.len();
        let unit_test_start_line = proof_harness_end_line + 1;
        let unit_test_end_line = unit_test_start_line + concrete_playback_num_lines - 1;
        let src_path = Path::new(src_path);
        let (path, file_name) = extract_parent_dir_and_src_file(src_path)?;
        let file_line_ranges = vec![FileLineRange {
            file: file_name,
            line_range: Some((unit_test_start_line, unit_test_end_line)),
        }];
        self.run_rustfmt(&file_line_ranges, Some(&path))?;
        Ok(())
    }

    /// Writes the new source code to a user's source file using a tempfile as the means.
    /// Returns whether the unit test was already in the old source code.
    fn add_test_inplace(
        &self,
        source_path: &str,
        proof_harness_end_line: usize,
        unit_test: &UnitTest,
    ) -> Result<bool> {
        // Read from source
        let source_file = File::open(source_path).unwrap();
        let source_reader = BufReader::new(source_file);

        // Create temp file
        let mut temp_file = TempFile::try_new("concrete_playback.tmp")?;
        let mut curr_line_num = 0;

        // Use a buffered reader/writer to generate the unit test line by line
        for line in source_reader.lines().flatten() {
            if line.contains(&unit_test.name) {
                if !self.args.common_args.quiet {
                    println!(
                        "Concrete playback unit test `{}/{}` already found in source code, so skipping modification.",
                        source_path, unit_test.name,
                    );
                }
                // the drop impl will take care of flushing and resetting
                return Ok(true);
            }
            curr_line_num += 1;
            if let Some(temp_writer) = temp_file.writer.as_mut() {
                writeln!(temp_writer, "{line}")?;
                if curr_line_num == proof_harness_end_line {
                    for unit_test_line in unit_test.code.iter() {
                        curr_line_num += 1;
                        writeln!(temp_writer, "{unit_test_line}")?;
                    }
                }
            }
        }

        temp_file.rename(source_path).expect("Could not rename file");
        Ok(false)
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

        if self.args.common_args.quiet {
            self.run_suppress(cmd).context("Failed to rustfmt modified source code.")?;
        } else {
            self.run_terminal(cmd).context("Failed to rustfmt modified source code")?;
        }
        Ok(())
    }
}

/// Generate a formatted unit test from a list of concrete values.
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
        format!("{:<4}let concrete_vals: Vec<Vec<u8>> = vec![", " "),
    ]
    .into_iter();
    let formatted_concrete_vals = format_concrete_vals(concrete_vals);
    let func_after_concrete_vals = [
        format!("{:<4}];", " "),
        format!("{:<4}kani::concrete_playback_run(concrete_vals, {harness_name});", " "),
        "}".to_string(),
    ]
    .into_iter();

    let full_func: Vec<_> = func_before_concrete_vals
        .chain(formatted_concrete_vals)
        .chain(func_after_concrete_vals)
        .collect();

    UnitTest { code: full_func, name: func_name }
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
            format!("{:<8}// {}", " ", concrete_val.interp_val),
            format!("{:<8}vec!{:?},", " ", concrete_val.byte_arr),
        ]
    })
}

/// Suppose `src_path` was `/path/to/file.txt`. This function extracts this into `/path/to` and `file.txt`.
fn extract_parent_dir_and_src_file(src_path: &Path) -> Result<(String, String)> {
    let parent_dir_as_path = src_path.parent().unwrap();
    let parent_dir = parent_dir_as_path.to_string_lossy().to_string();
    let src_file_name_as_osstr = src_path.file_name();
    let src_file = src_file_name_as_osstr.unwrap().to_string_lossy().to_string();
    Ok((parent_dir, src_file))
}

struct FileLineRange {
    file: String,
    line_range: Option<(usize, usize)>,
}

struct UnitTest {
    code: Vec<String>,
    name: String,
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
    pub fn extract_harness_values(result_items: &[Property]) -> Vec<Vec<ConcreteVal>> {
        result_items
            .iter()
            .filter(|prop| {
                (prop.property_class() == "assertion" && prop.status == CheckStatus::Failure)
                    || (prop.property_class() == "cover" && prop.status == CheckStatus::Satisfied)
            })
            .map(|property| {
                // Extract values for the first assertion that has failed.
                let trace = property
                    .trace
                    .as_ref()
                    .expect(&format!("Missing trace for {}", property.property_name()));
                let concrete_vals: Vec<ConcreteVal> =
                    trace.iter().filter_map(&extract_from_trace_item).collect();

                concrete_vals
            })
            .collect()
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

#[cfg(test)]
mod tests {
    use super::concrete_vals_extractor::*;
    use super::*;
    use crate::cbmc_output_parser::{
        CheckStatus, Property, PropertyId, SourceLocation, TraceData, TraceItem, TraceValue,
    };

    /// util function for unit tests taht generates the rustfmt args used for formatting specific lines inside specific files.
    /// note - adding this within the test mod because it gives a lint warning without it.
    fn rustfmt_args(file_line_ranges: &[FileLineRange]) -> Vec<OsString> {
        let mut args: Vec<OsString> = Vec::new();
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
        args
    }

    #[test]
    fn format_zero_concrete_vals() {
        let concrete_vals: [ConcreteVal; 0] = [];
        let actual: Vec<_> = format_concrete_vals(&concrete_vals).collect();
        let expected: Vec<String> = Vec::new();
        assert_eq!(actual, expected);
    }

    /// Check that the generated unit tests have the right formatting and indentation
    #[test]
    fn format_two_concrete_vals() {
        let concrete_vals = [
            ConcreteVal { byte_arr: vec![0, 0], interp_val: "0".to_string() },
            ConcreteVal { byte_arr: vec![0, 0, 0, 0, 0, 0, 0, 0], interp_val: "0l".to_string() },
        ];
        let actual: Vec<_> = format_concrete_vals(&concrete_vals).collect();
        let expected = vec![
            format!("{:<8}// 0", " "),
            format!("{:<8}vec![0, 0],", " "),
            format!("{:<8}// 0l", " "),
            format!("{:<8}vec![0, 0, 0, 0, 0, 0, 0, 0],", " "),
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
        let last_underscore_idx = underscore_locs.last().unwrap().0;
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
        let full_func = unit_test.code;
        let split_unit_test_name = split_unit_test_name(&unit_test.name);
        let expected_after_func_name = vec![
            format!("{:<4}let concrete_vals: Vec<Vec<u8>> = vec![", " "),
            format!("{:<8}// 0", " "),
            format!("{:<8}vec![0, 0],", " "),
            format!("{:<4}];", " "),
            format!("{:<4}kani::concrete_playback_run(concrete_vals, {harness_name});", " "),
            "}".to_string(),
        ];

        assert_eq!(full_func[0], "#[test]");
        assert_eq!(
            split_unit_test_name.before_hash,
            format!("kani_concrete_playback_{harness_name}")
        );
        assert_eq!(full_func[1], format!("fn {}() {{", unit_test.name));
        assert_eq!(full_func[2..], expected_after_func_name);
    }

    /// Generates a unit test and returns its hash.
    fn extract_hash_from_unit_test(harness_name: &str, concrete_vals: &[ConcreteVal]) -> String {
        let unit_test = format_unit_test(harness_name, concrete_vals);
        split_unit_test_name(&unit_test.name).hash
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
    fn check_rustfmt_args_no_line_ranges() {
        let file_line_ranges = [FileLineRange { file: "file1".to_string(), line_range: None }];
        let args = rustfmt_args(&file_line_ranges);
        let expected: Vec<OsString> = vec!["file1".into()];
        assert_eq!(args, expected);
    }

    #[test]
    fn check_rustfmt_args_some_line_ranges() {
        let file_line_ranges = [
            FileLineRange { file: "file1".to_string(), line_range: None },
            FileLineRange { file: "path/to/file2".to_string(), line_range: Some((1, 3)) },
        ];
        let args = rustfmt_args(&file_line_ranges);
        let expected: Vec<OsString> = [
            "--unstable-features",
            "--file-lines",
            "[{\"file\":\"path/to/file2\",\"range\":[1,3]}]",
            "file1",
            "path/to/file2",
        ]
        .into_iter()
        .map(|arg| arg.into())
        .collect();
        assert_eq!(args, expected);
    }

    #[test]
    fn check_extract_parent_dir_and_src_file() {
        let src_path = "/path/to/file.txt";
        let src_path = Path::new(src_path);
        let (path, file_name) = extract_parent_dir_and_src_file(src_path).unwrap();
        assert_eq!(path, "/path/to");
        assert_eq!(file_name, "file.txt");
    }

    /// Test util functions which extract the counter example values from a property.
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
