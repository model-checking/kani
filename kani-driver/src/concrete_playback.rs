// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module for parsing concrete values from CBMC output traces,
//! generating concrete playback unit tests, and adding them to the user's source code.

use crate::args::ConcretePlaybackMode;
use crate::call_cbmc::VerificationStatus;
use crate::cbmc_output_parser::VerificationOutput;
use crate::session::KaniSession;
use anyhow::{ensure, Context, Result};
use concrete_vals_extractor::{extract_from_processed_items, ConcreteVal};
use kani_metadata::HarnessMetadata;
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, BufWriter, Write};
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
                    harness.mangled_name,
                    concrete_playback.full_func.join("\n")
                );
                println!(
                    "INFO: To automatically add the concrete playback unit test `{}` to the src code, run Kani with `--concrete-playback=InPlace`.",
                    &concrete_playback.func_name
                );
            }

            if *playback_mode == ConcretePlaybackMode::InPlace {
                if !self.args.quiet {
                    println!(
                        "INFO: Now modifying the source code to include the concrete playback unit test `{}`.",
                        &concrete_playback.func_name
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
        src_path_as_str: &str,
        proof_harness_end_line: usize,
        concrete_playback: &UnitTest,
    ) -> Result<()> {
        let tmp_src_path = src_path_as_str.to_string() + ".concrete_playback_overwrite";
        let unit_test_already_in_src = write_new_src_to_tmp_file(
            src_path_as_str,
            &tmp_src_path,
            proof_harness_end_line,
            concrete_playback,
        )?;

        if unit_test_already_in_src {
            if !self.args.quiet {
                println!(
                    "Concrete playback unit test `{}/{}` already found in source code, so skipping modification.",
                    src_path_as_str, concrete_playback.func_name,
                );
            }
            fs::remove_file(&tmp_src_path)
                .with_context(|| format!("Couldn't remove tmp src file `{tmp_src_path}`."))?;
            return Ok(());
        }

        // Renames are usually automic, so we won't corrupt the user's source file during a crash.
        fs::rename(&tmp_src_path, src_path_as_str).with_context(|| {
            format!(
                "Couldn't rename tmp src file `{tmp_src_path}` to actual src file `{src_path_as_str}`."
            )
        })?;

        // Run rustfmt on just the inserted lines.
        let concrete_playback_num_lines = concrete_playback.full_func.len();
        let unit_test_start_line = proof_harness_end_line + 1;
        let unit_test_end_line = unit_test_start_line + concrete_playback_num_lines - 1;
        let src_path = Path::new(src_path_as_str);
        let parent_dir_and_src_file = extract_parent_dir_and_src_file(src_path)?;
        let file_line_ranges = vec![FileLineRange {
            file: parent_dir_and_src_file.src_file,
            line_range: Some((unit_test_start_line, unit_test_end_line)),
        }];
        self.run_rustfmt(&file_line_ranges, Some(&parent_dir_and_src_file.parent_dir))?;
        Ok(())
    }

    /// Run rustfmt on the given src file, and optionally on only the specific lines.
    fn run_rustfmt(
        &self,
        file_line_ranges: &[FileLineRange],
        current_dir_opt: Option<&str>,
    ) -> Result<()> {
        let mut cmd = Command::new("rustfmt");
        cmd.args(rustfmt_args(file_line_ranges));
        if let Some(current_dir) = current_dir_opt {
            cmd.current_dir(current_dir);
        }
        if self.args.quiet { self.run_suppress(cmd) } else { self.run_terminal(cmd) }
            .context("Failed to rustfmt modified source code.")?;
        Ok(())
    }

    /// Helper function to inform the user that they tried to generate concrete playback unit tests when there were no failing harnesses.
    pub fn inform_if_no_failed(&self, failed_harnesses: &[&HarnessMetadata]) {
        if self.args.concrete_playback.is_some() && !self.args.quiet && failed_harnesses.is_empty()
        {
            println!(
                "INFO: The concrete playback feature never generated unit tests because there were no failing harnesses."
            )
        }
    }
}

/// Writes the new source code to a temporary file. Returns whether the unit test was already in the old source code.
fn write_new_src_to_tmp_file(
    src_path_as_str: &str,
    tmp_src_path: &str,
    proof_harness_end_line: usize,
    concrete_playback: &UnitTest,
) -> Result<bool> {
    let src_file = File::open(src_path_as_str)
        .with_context(|| format!("Couldn't open user's source code file `{src_path_as_str}`"))?;
    let src_buf_reader = BufReader::new(src_file);
    let tmp_src_file = File::create(&tmp_src_path)
        .with_context(|| format!("Couldn't create tmp source code file `{}`", tmp_src_path))?;
    let mut tmp_src_buf_writer = BufWriter::new(tmp_src_file);
    let mut unit_test_already_in_src = false;
    let mut curr_line_num = 0;

    for line in src_buf_reader.lines().flatten() {
        if line.contains(&concrete_playback.func_name) {
            unit_test_already_in_src = true;
        }
        curr_line_num += 1;
        writeln!(tmp_src_buf_writer, "{}", line)?;
        if curr_line_num == proof_harness_end_line {
            for unit_test_line in concrete_playback.full_func.iter() {
                curr_line_num += 1;
                writeln!(tmp_src_buf_writer, "{}", unit_test_line)?;
            }
        }
    }

    // Flush before we remove/rename the file.
    tmp_src_buf_writer.flush()?;
    Ok(unit_test_already_in_src)
}

struct UnitTest {
    full_func: Vec<String>,
    func_name: String,
}

const TAB: &str = "    ";

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
        format!("{TAB}let concrete_vals: Vec<Vec<u8>> = vec!["),
    ]
    .into_iter();
    let formatted_concrete_vals = format_concrete_vals(concrete_vals);
    let func_after_concrete_vals = [
        format!("{TAB}];"),
        format!("{TAB}kani::concrete_playback_run(concrete_vals, {harness_name});"),
        "}".to_string(),
    ]
    .into_iter();

    let full_func: Vec<_> = func_before_concrete_vals
        .chain(formatted_concrete_vals)
        .chain(func_after_concrete_vals)
        .collect();
    UnitTest { full_func, func_name }
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
    let two_tab = TAB.repeat(2);
    concrete_vals.iter().flat_map(move |concrete_val| {
        [
            format!("{two_tab}// {}", concrete_val.interp_val),
            format!("{two_tab}vec!{:?},", concrete_val.byte_arr),
        ]
    })
}

struct ParentDirAndSrcFile {
    parent_dir: String,
    src_file: String,
}

/// Suppose `src_path` was `/path/to/file.txt`. This function extracts this into `/path/to` and `file.txt`.
fn extract_parent_dir_and_src_file(src_path: &Path) -> Result<ParentDirAndSrcFile> {
    let parent_dir_as_path = src_path.parent().with_context(|| {
        format!("Expected source file `{}` to be in a directory", src_path.display())
    })?;
    let parent_dir = parent_dir_as_path.to_str().with_context(|| {
        format!(
            "Couldn't convert source file parent directory `{}` from  str",
            parent_dir_as_path.display()
        )
    })?;
    let src_file_name_as_osstr = src_path.file_name().with_context(|| {
        format!("Couldn't get the file name from the source file `{}`", src_path.display())
    })?;
    let src_file = src_file_name_as_osstr.to_str().with_context(|| {
        format!(
            "Couldn't convert source code file name `{:?}` from OsStr to str",
            src_file_name_as_osstr
        )
    })?;
    Ok(ParentDirAndSrcFile { parent_dir: parent_dir.to_string(), src_file: src_file.to_string() })
}

struct FileLineRange {
    file: String,
    line_range: Option<(usize, usize)>,
}

/// Generates the rustfmt args used for formatting specific lines inside specific files.
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

/// Extract concrete values from the CBMC output processed items.
/// Note: we extract items that roughly look like the following:
/// ```json
/// ...
/// { "result": [
///     ...,
///     { "property": "assertion.1", "status": "FAILURE", "trace": [
///         ...,
///         { "lhs": "goto_symex$$return_value...", "stepType": "assignment",
///           "sourceLocation": { "function": "kani::any_raw_internal::<u8, 1_usize>" },
///           "value": { "binary": "00000001", "data": "1", "width": 8 } }
///         ..., ] }
///     ..., ] }
/// ```
mod concrete_vals_extractor {
    use crate::cbmc_output_parser::{
        extract_property_class, CheckStatus, ParserItem, Property, TraceItem,
    };
    use anyhow::{bail, ensure, Context, Result};

    #[derive(Hash)]
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

#[cfg(test)]
mod tests {
    use super::concrete_vals_extractor::*;
    use super::*;
    use crate::cbmc_output_parser::{
        CheckStatus, ParserItem, Property, SourceLocation, TraceData, TraceItem, TraceValue,
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
        let two_tab = TAB.repeat(2);
        let expected = vec![
            format!("{two_tab}// 0"),
            format!("{two_tab}vec![0, 0],"),
            format!("{two_tab}// 0l"),
            format!("{two_tab}vec![0, 0, 0, 0, 0, 0, 0, 0],"),
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
        let full_func = unit_test.full_func;
        let split_unit_test_name = split_unit_test_name(&unit_test.func_name);
        let two_tab = TAB.repeat(2);
        let expected_after_func_name = vec![
            format!("{TAB}let concrete_vals: Vec<Vec<u8>> = vec!["),
            format!("{two_tab}// 0"),
            format!("{two_tab}vec![0, 0],"),
            format!("{TAB}];"),
            format!("{TAB}kani::concrete_playback_run(concrete_vals, {harness_name});"),
            "}".to_string(),
        ];

        assert_eq!(full_func[0], "#[test]");
        assert_eq!(
            split_unit_test_name.before_hash,
            format!("kani_concrete_playback_{harness_name}")
        );
        assert_eq!(full_func[1], format!("fn {}() {{", unit_test.func_name));
        assert_eq!(full_func[2..], expected_after_func_name);
    }

    /// Generates a unit test and returns its hash.
    fn extract_hash_from_unit_test(harness_name: &str, concrete_vals: &[ConcreteVal]) -> String {
        let unit_test = format_unit_test(harness_name, concrete_vals);
        split_unit_test_name(&unit_test.func_name).hash
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
        let src_path_as_str = "/path/to/file.txt";
        let src_path = Path::new(src_path_as_str);
        let parent_dir_and_src_file = extract_parent_dir_and_src_file(src_path).unwrap();
        assert_eq!(parent_dir_and_src_file.parent_dir, "/path/to");
        assert_eq!(parent_dir_and_src_file.src_file, "file.txt");
    }

    #[test]
    fn check_concrete_vals_extractor() {
        let processed_items = [ParserItem::Result {
            result: vec![Property {
                description: "".to_string(),
                property: "assertion.1".to_string(),
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
            }],
        }];
        let concrete_vals = extract_from_processed_items(&processed_items).unwrap();
        let concrete_val = &concrete_vals[0];

        assert_eq!(concrete_val.byte_arr, vec![1, 3]);
        assert_eq!(concrete_val.interp_val, "385");
    }
}
