// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module for parsing CBMC's JSON output. In general, this output follows
//! the structure:
//! ```
//! [
//!     Program,
//!     Message,
//!     ...,
//!     Message,
//!     Result,
//!     Message,
//!     ProverStatus
//! ]
//! ```
//! The parser included in this file reads from buffered input line by line, and
//! determines if an item can be processed after reading certain lines.
//!
//! The rest of code in this file is related to result postprocessing.

use crate::{args::OutputFormat, call_cbmc::VerificationStatus};
use anyhow::Result;
use pathdiff::diff_paths;
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, ChildStdout},
};
use structopt::lazy_static::lazy_static;

lazy_static! {
    /// Hash map that relates property classes with descriptions, used by
    /// `get_readable_description` to provide user friendly descriptions.
    /// See the comment in `get_readable_description` for more information on
    /// how this data structure is used.
    static ref CBMC_ALT_DESCRIPTIONS: HashMap<&'static str, Vec<(&'static str, Option<&'static str>)>> = {
        let mut map = HashMap::new();
        map.insert("error_label", vec![]);
        map.insert("division-by-zero", vec![("division by zero", None)]);
        map.insert("enum-range-check", vec![("enum range check", None)]);
        map.insert(
            "undefined-shift",
            vec![
                ("shift distance is negative", None),
                ("shift distance too large", None),
                ("shift operand is negative", None),
                ("shift of non-integer type", None),
            ],
        );
        map.insert(
            "overflow",
            vec![
                ("result of signed mod is not representable", None),
                ("arithmetic overflow on signed type conversion", None),
                ("arithmetic overflow on signed division", None),
                ("arithmetic overflow on signed unary minus", None),
                ("arithmetic overflow on signed shl", None),
                ("arithmetic overflow on unsigned unary minus", None),
                ("arithmetic overflow on signed +", Some("arithmetic overflow on signed addition")),
                (
                    "arithmetic overflow on signed -",
                    Some("arithmetic overflow on signed subtraction"),
                ),
                (
                    "arithmetic overflow on signed *",
                    Some("arithmetic overflow on signed multiplication"),
                ),
                (
                    "arithmetic overflow on unsigned +",
                    Some("arithmetic overflow on unsigned addition"),
                ),
                (
                    "arithmetic overflow on unsigned -",
                    Some("arithmetic overflow on unsigned subtraction"),
                ),
                (
                    "arithmetic overflow on unsigned *",
                    Some("arithmetic overflow on unsigned multiplication"),
                ),
                ("arithmetic overflow on floating-point typecast", None),
                ("arithmetic overflow on floating-point division", None),
                ("arithmetic overflow on floating-point addition", None),
                ("arithmetic overflow on floating-point subtraction", None),
                ("arithmetic overflow on floating-point multiplication", None),
                ("arithmetic overflow on unsigned to signed type conversion", None),
                ("arithmetic overflow on float to signed integer type conversion", None),
                ("arithmetic overflow on signed to unsigned type conversion", None),
                ("arithmetic overflow on unsigned to unsigned type conversion", None),
                ("arithmetic overflow on float to unsigned integer type conversion", None),
            ],
        );
        map.insert(
            "NaN",
            vec![
                ("NaN on +", Some("NaN on addition")),
                ("NaN on -", Some("NaN on subtraction")),
                ("NaN on /", Some("NaN on division")),
                ("NaN on *", Some("NaN on multiplication")),
            ],
        );
        map.insert("pointer", vec![("same object violation", None)]);
        map.insert(
            "pointer_arithmetic",
            vec![
                ("pointer relation: deallocated dynamic object", None),
                ("pointer relation: dead object", None),
                ("pointer relation: pointer NULL", None),
                ("pointer relation: pointer invalid", None),
                ("pointer relation: pointer outside dynamic object bounds", None),
                ("pointer relation: pointer outside object bounds", None),
                ("pointer relation: invalid integer address", None),
                ("pointer arithmetic: deallocated dynamic object", None),
                ("pointer arithmetic: dead object", None),
                ("pointer arithmetic: pointer NULL", None),
                ("pointer arithmetic: pointer invalid", None),
                ("pointer arithmetic: pointer outside dynamic object bounds", None),
                ("pointer arithmetic: pointer outside object bounds", None),
                ("pointer arithmetic: invalid integer address", None),
            ],
        );
        map.insert(
            "pointer_dereference",
            vec![
                (
                    "dereferenced function pointer must be",
                    Some("dereference failure: invalid function pointer"),
                ),
                ("dereference failure: pointer NULL", None),
                ("dereference failure: pointer invalid", None),
                ("dereference failure: deallocated dynamic object", None),
                ("dereference failure: dead object", None),
                ("dereference failure: pointer outside dynamic object bounds", None),
                ("dereference failure: pointer outside object bounds", None),
                ("dereference failure: invalid integer address", None),
            ],
        );
        // These are very hard to understand without more context.
        map.insert(
            "pointer_primitives",
            vec![
                ("pointer invalid", None),
                ("deallocated dynamic object", Some("pointer to deallocated dynamic object")),
                ("dead object", Some("pointer to dead object")),
                ("pointer outside dynamic object bounds", None),
                ("pointer outside object bounds", None),
                ("invalid integer address", None),
            ],
        );
        map.insert(
            "array_bounds",
            vec![
                ("lower bound", Some("index out of bounds")),
                // This one is redundant:
                // ("dynamic object upper bound", Some("access out of bounds")),
                (
                    "upper bound",
                    Some(
                        "index out of bounds: the length is less than or equal to the given index",
                    ),
                ),
            ],
        );
        map.insert(
            "bit_count",
            vec![
                ("count trailing zeros is undefined for value zero", None),
                ("count leading zeros is undefined for value zero", None),
            ],
        );
        map.insert("memory-leak", vec![("dynamically allocated memory never freed", None)]);
        // These pre-conditions should not print temporary variables since they are embedded in the libc implementation.
        // They are added via `__CPROVER_precondition`.
        // map.insert("precondition_instance": vec![]);
        map
    };
}

const UNSUPPORTED_CONSTRUCT_DESC: &str = "is not currently supported by Kani";
const UNWINDING_ASSERT_DESC: &str = "unwinding assertion loop";
const ASSERTION_FALSE: &str = "assertion false";
const DEFAULT_ASSERTION: &str = "assertion";
const REACH_CHECK_DESC: &str = "[KANI_REACHABILITY_CHECK]";

/// Enum that represents a parser item.
/// See the parser for more information on how they are processed.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ParserItem {
    Program {
        program: String,
    },
    #[serde(rename_all = "camelCase")]
    Message {
        message_text: String,
        message_type: String,
    },
    Result {
        result: Vec<Property>,
    },
    #[serde(rename_all = "camelCase")]
    ProverStatus {
        _c_prover_status: String,
    },
}

impl ParserItem {
    /// Determines if an item must be skipped or not.
    fn must_be_skipped(&self) -> bool {
        matches!(&self, ParserItem::Message { message_text, .. } if message_text.starts_with("Building error trace"))
            || matches!(&self, ParserItem::Message { message_text, .. } if message_text.starts_with("VERIFICATION"))
    }
}

/// Struct that represents a property.
///
/// Note: `reach` is not part of the parsed data, but it's useful to annotate
/// its reachability status.
#[derive(Clone, Debug, Deserialize)]
pub struct Property {
    pub description: String,
    pub property: String,
    #[serde(rename = "sourceLocation")]
    pub source_location: SourceLocation,
    pub status: CheckStatus,
    pub reach: Option<CheckStatus>,
    pub trace: Option<Vec<TraceItem>>,
}

/// Struct that represents a source location.
///
/// Source locations may be completely empty, which is why
/// all members are optional.
#[derive(Clone, Debug, Deserialize)]
pub struct SourceLocation {
    pub column: Option<String>,
    pub file: Option<String>,
    pub function: Option<String>,
    pub line: Option<String>,
}

impl SourceLocation {
    /// Determines if fundamental parts of a source location are missing.
    fn is_missing(&self) -> bool {
        self.file.is_none() && self.function.is_none()
    }
}

/// `Display` implement for `SourceLocation`.
///
/// This is used to format source locations for individual checks. But source
/// locations may be printed in a different way in other places (e.g., in the
/// "Failed Checks" summary at the end).
impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fmt_str = String::new();
        if self.file.is_some() {
            let file_str = format!("{}", filepath(self.file.clone().unwrap()));
            fmt_str.push_str(file_str.as_str());
            if self.line.is_some() {
                let line_str = format!(":{}", self.line.clone().unwrap());
                fmt_str.push_str(line_str.as_str());
                if self.column.is_some() {
                    let column_str = format!(":{}", self.column.clone().unwrap());
                    fmt_str.push_str(column_str.as_str());
                }
            }
        } else {
            fmt_str.push_str("Unknown File");
        }
        if self.function.is_some() {
            let fun_str = format!(" in function {}", self.function.clone().unwrap());
            fmt_str.push_str(fun_str.as_str());
        }

        write! {f, "{}", fmt_str}
    }
}

/// Returns a path relative to the current working directory.
fn filepath(file: String) -> String {
    let file_path = PathBuf::from(file.clone());
    let cur_dir = env::current_dir().unwrap();

    let diff_path = diff_paths(file_path, cur_dir);
    if diff_path.is_some() {
        diff_path.unwrap().into_os_string().into_string().unwrap()
    } else {
        file
    }
}
/// Struct that represents traces.
///
/// In general, traces may include more information than this, but this is not
/// documented anywhere. So we ignore the rest for now.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceItem {
    pub thread: u32,
    pub step_type: String,
    pub hidden: bool,
    pub source_location: Option<SourceLocation>,
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CheckStatus {
    Failure,
    Success,
    Undetermined,
    Unreachable,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let check_str = match self {
            CheckStatus::Success => "SUCCESS",
            CheckStatus::Failure => "FAILURE",
            CheckStatus::Unreachable => "UNREACHABLE",
            CheckStatus::Undetermined => "UNDETERMINED",
        };
        write! {f, "{}", check_str}
    }
}

#[derive(PartialEq)]
enum Action {
    ClearInput,
    ProcessItem,
}

/// A parser for CBMC output, whose state is determined by:
///  1. The input accumulator, required to process items on the fly.
///  2. The buffer, which is accessed to retrieve more lines.
///
/// CBMC's JSON output is defined as a JSON array which contains:
///  1. One program at the beginning (i.e., a message with CBMC's version).
///  2. Messages, which can appears anywhere, and are either status messages or error messages.
///  3. Verification results, another JSON array with all individual checks.
///  4. Prover status, at the end. Because the verification results depends on
///     our postprocessing, this is not used.
///
/// The parser reads the output line by line. A line may trigger one action, and
/// the action may return a parsed item.
struct Parser<'a, 'b> {
    pub input_so_far: String,
    pub buffer: &'a mut BufReader<&'b mut ChildStdout>,
}

impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(buffer: &'a mut BufReader<&'b mut ChildStdout>) -> Self {
        Parser { input_so_far: String::new(), buffer: buffer }
    }

    /// Triggers an action based on the input:
    ///  * Square brackets ('[' and ']') will trigger the `ClearInput` action
    ///    because we assume parsing is done on a JSON array.
    ///  * Curly closing bracket ('}') preceded by two spaces will trigger the
    ///    `ProcessItem` action.
    fn triggers_action(&self, input: String) -> Option<Action> {
        if input.starts_with("[") || input.starts_with("]") {
            return Some(Action::ClearInput);
        }
        if input.starts_with("  }") {
            return Some(Action::ProcessItem);
        }
        None
    }

    /// Clears the input accumulated so far.
    fn clear_input(&mut self) {
        self.input_so_far = String::new();
    }

    /// Performs an action. In both cases, the input is cleared.
    fn do_action(&mut self, action: Action) -> Option<ParserItem> {
        match action {
            Action::ClearInput => {
                self.clear_input();
                None
            }
            Action::ProcessItem => {
                let item = self.parse_item();
                self.clear_input();
                Some(item)
            }
        }
    }

    // Adds a string to the input accumulated so far
    fn add_to_input(&mut self, input: String) {
        self.input_so_far.push_str(input.as_str());
    }

    // Returns a `ParserItem` from the input we have accumulated so far. Since
    // all items except the last one are delimited (with a comma), we first try
    // to parse the item without the delimiter (i.e., the last character). If
    // that fails, then we parse the item using the whole input.
    fn parse_item(&self) -> ParserItem {
        let string_without_delimiter = &self.input_so_far.as_str()[0..self.input_so_far.len() - 2];
        let block: Result<ParserItem, _> = serde_json::from_str(string_without_delimiter);
        if block.is_ok() {
            return block.unwrap();
        }
        let complete_string = &self.input_so_far.as_str()[0..self.input_so_far.len()];
        let block: Result<ParserItem, _> = serde_json::from_str(complete_string);
        assert!(block.is_ok());
        block.unwrap()
    }

    /// Processes a line to determine if an action must be triggered.
    /// The action may result in a `ParserItem`, which is then returned.
    pub fn process_line(&mut self, input: String) -> Option<ParserItem> {
        self.add_to_input(input.clone());
        let action_required = self.triggers_action(input.clone());
        if action_required.is_some() {
            let action = action_required.unwrap();
            let possible_item = self.do_action(action);
            return possible_item;
        }
        None
    }
}

/// The iterator implementation for `Parser` reads the buffer line by line,
/// and determines if it must return an item based on processing each line.
impl<'a, 'b> Iterator for Parser<'a, 'b> {
    type Item = ParserItem;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut input = String::new();
            match self.buffer.read_line(&mut input) {
                Ok(len) => {
                    if len == 0 {
                        return None;
                    }
                    let item = self.process_line(input);
                    if item.is_some() {
                        return item;
                    } else {
                        continue;
                    }
                }
                Err(error) => {
                    panic!("Error: Got error {} while parsing the output.", error);
                }
            }
        }
    }
}

/// Processes a `ParserItem`. In general, all items are returned as they are,
/// except for:
///  * Error messages, which may be edited.
///  * Verification results, which must be postprocessed.
fn process_item(
    item: ParserItem,
    extra_ptr_checks: bool,
    verification_result: &mut bool,
) -> ParserItem {
    match item {
        ParserItem::Result { result } => {
            let (postprocessed_result, overall_status) =
                postprocess_result(result, extra_ptr_checks);
            *verification_result = overall_status;
            ParserItem::Result { result: postprocessed_result }
        }
        ParserItem::Message { ref message_type, .. } if message_type == "ERROR" => {
            postprocess_error_message(item)
        }
        item => item,
    }
}

/// Edits an error message.
///
/// At present, we only know one case where CBMC emits an error message, related
/// to `--object-bits` being too low. The message is edited to show Kani
/// options.
fn postprocess_error_message(message: ParserItem) -> ParserItem {
    if let ParserItem::Message { ref message_text, message_type: _ } = message && message_text.contains("use the `--object-bits n` option") {
        ParserItem::Message {
            message_text: message_text.replace("--object-bits ", "--enable-unstable --cbmc-args --object-bits "),
            message_type: String::from("ERROR") }
    } else {
        message
    }
}

pub fn process_cbmc_output(
    mut cmd: Child,
    extra_ptr_checks: bool,
    output_format: &OutputFormat,
) -> VerificationStatus {
    let stdout = cmd.stdout.as_mut().unwrap();
    let mut stdout_reader = BufReader::new(stdout);
    let parser = Parser::new(&mut stdout_reader);
    let mut result = false;

    for item in parser {
        // Some items (e.g., messages) are skipped.
        // We could also process them and decide to skip later.
        if item.must_be_skipped() {
            continue;
        }
        let processed_item = process_item(item, extra_ptr_checks, &mut result);
        // Both formatting and printing could be handled by objects which
        // implement a trait `Printer`.
        let formatted_item = format_item(&processed_item, &output_format);
        if formatted_item.is_some() {
            println!("{}", formatted_item.unwrap())
        };
        // TODO: Record processed items and dump them into a JSON file
        // <https://github.com/model-checking/kani/issues/942>
    }
    if result { VerificationStatus::Success } else { VerificationStatus::Failure }
}

/// Returns an optional formatted item based on the output format
fn format_item(item: &ParserItem, output_format: &OutputFormat) -> Option<String> {
    match output_format {
        OutputFormat::Old => todo!(),
        OutputFormat::Regular => format_item_regular(item),
        OutputFormat::Terse => format_item_terse(item),
    }
}

/// Formats an item using the regular output format
fn format_item_regular(item: &ParserItem) -> Option<String> {
    match item {
        ParserItem::Program { program } => Some(format!("{}", program)),
        ParserItem::Message { message_text, .. } => Some(format!("{}", message_text)),
        ParserItem::Result { result } => Some(format_result(result, true)),
        _ => None,
    }
}

/// Formats an item using the terse output format
fn format_item_terse(item: &ParserItem) -> Option<String> {
    match item {
        ParserItem::Result { result } => Some(format_result(result, false)),
        _ => None,
    }
}

/// Formats a result item (i.e., the complete set of verification checks).
/// This could be split into two functions for clarity, but at the moment
/// it uses the flag `show_checks` which depends on the output format.
fn format_result(properties: &Vec<Property>, show_checks: bool) -> String {
    let mut result_str = String::new();
    let mut number_tests_failed = 0;
    let mut number_tests_unreachable = 0;
    let mut number_tests_undetermined = 0;
    let mut failed_tests: Vec<&Property> = vec![];

    let mut index = 1;

    if show_checks {
        result_str.push_str("\nRESULTS:\n");
    }

    for prop in properties {
        let name = &prop.property;
        let status = &prop.status;
        let description = &prop.description;
        let location = &prop.source_location;

        match status {
            CheckStatus::Failure => {
                number_tests_failed += 1;
                failed_tests.push(&prop);
            }
            CheckStatus::Undetermined => {
                number_tests_undetermined += 1;
            }
            CheckStatus::Unreachable => {
                number_tests_unreachable += 1;
            }
            _ => (),
        }

        if show_checks {
            // TODO: Add color to status if printing to terminal.
            // <TODO_URL>
            let check_id = format!("Check {}: {}\n", index, name);
            let status_msg = format!("\t - Status: {}\n", status);
            let descrition_msg = format!("\t - Description: \"{}\"\n", description);

            result_str.push_str(check_id.as_str());
            result_str.push_str(status_msg.as_str());
            result_str.push_str(descrition_msg.as_str());

            if !location.is_missing() {
                let location_msg = format!("\t - Location: {}\n", location);
                result_str.push_str(location_msg.as_str());
            }
            result_str.push_str("\n");
        }

        index += 1;
    }

    if show_checks {
        result_str.push_str("\nSUMMARY:");
    } else {
        result_str.push_str("\nVERIFICATION RESULT:");
    }

    let summary = format!("\n ** {} of {} failed", number_tests_failed, properties.len());
    result_str.push_str(summary.as_str());

    let mut other_status = Vec::<String>::new();
    if number_tests_undetermined > 0 {
        let undetermined_str = format!("{} undetermined", number_tests_undetermined);
        other_status.push(undetermined_str);
    }
    if number_tests_unreachable > 0 {
        let unreachable_str = format!("{} unreachable", number_tests_unreachable);
        other_status.push(unreachable_str);
    }
    if other_status.len() > 0 {
        result_str.push_str(" (");
        result_str.push_str(&other_status.join(","));
        result_str.push_str(")");
    }
    result_str.push_str("\n");

    for prop in failed_tests {
        let failure_message = build_failure_message(prop.description.clone(), &prop.trace.clone());
        result_str.push_str(failure_message.as_str());
    }

    let verification_result = if number_tests_failed == 0 { "SUCCESSFUL " } else { "FAILED" };
    let overall_result = format!("\nVERIFICATION:- {}\n", verification_result);
    result_str.push_str(overall_result.as_str());

    // Ideally, we should generate two `ParserItem::Message` and push them
    // into the parser iterator so they are the next messages to be processed.
    // However, we haven't figured out the best way to do this for now.
    // <TODO_URL>
    if has_check_failure(&properties, UNSUPPORTED_CONSTRUCT_DESC) {
        result_str.push_str(
            "** WARNING: A Rust construct that is not currently supported \
        by Kani was found to be reachable. Check the results for \
        more details.",
        );
    }
    if has_check_failure(&properties, UNWINDING_ASSERT_DESC) {
        result_str.push_str("[Kani] info: Verification output shows one or more unwinding failures.\n\
        [Kani] tip: Consider increasing the unwinding value or disabling `--unwinding-assertions`.\n");
    }

    result_str
}

/// Attempts to build a message for a failed property with as much detailed
/// information on the source location as possible.
fn build_failure_message(description: String, trace: &Option<Vec<TraceItem>>) -> String {
    let backup_failure_message = format!("Failed Checks: {}\n", description);
    if trace.is_none() {
        return backup_failure_message;
    }
    let failure_trace = trace.clone().unwrap();

    let failure_source_wrap = failure_trace[failure_trace.len() - 1].source_location.clone();
    if failure_source_wrap.is_none() {
        return backup_failure_message;
    }
    let failure_source = failure_source_wrap.unwrap();

    if failure_source.file.is_some()
        && failure_source.function.is_some()
        && failure_source.line.is_some()
    {
        let failure_file = failure_source.file.unwrap();
        let failure_function = failure_source.function.unwrap();
        let failure_line = failure_source.line.unwrap();
        return format!(
            "Failed Checks: {}\n File: \"{}\", line {}, in {}\n",
            description, failure_file, failure_line, failure_function
        );
    }
    backup_failure_message
}

/// Postprocess verification results to check for certain cases (e.g. a reachable unsupported construct or a failed
/// unwinding assertion), and update the results of impacted checks accordingly.
///
/// This postprocessing follows the same steps:
///     1. Change all "SUCCESS" results to "UNDETERMINED" if the reachability check
///     for a Rust construct that is not currently supported by Kani failed, since
///     the missing exploration of execution paths through the unsupported construct
///     may hide failures
///     2. Change a check's result from "SUCCESS" to "UNREACHABLE" if its
///     reachability check's result was "SUCCESS"
///     3. Change results from "SUCCESS" to "UNDETERMINED" if an unwinding
///     assertion failed, since the insufficient unwinding may cause some execution
///     paths to be left unexplored.
///
///     Additionally, print a message at the end of the output that indicates if any
///     of the special cases above was hit.
pub fn postprocess_result(
    properties: Vec<Property>,
    extra_ptr_checks: bool,
) -> (Vec<Property>, bool) {
    // First, determine if there are reachable unsupported constructs or unwinding assertions
    let has_reachable_unsupported_constructs =
        has_check_failure(&properties, UNSUPPORTED_CONSTRUCT_DESC);
    let has_failed_unwinding_asserts = has_check_failure(&properties, UNWINDING_ASSERT_DESC);
    // println!("properties: {:?}\n", properties);
    // Then, determine if there are reachable undefined functions, and change
    // their description to highlight this fact
    let (properties_with_undefined, has_reachable_undefined_functions) =
        modify_undefined_function_checks(properties);
    // println!("properties_with_undefined: {:?}\n", properties_with_undefined);
    // Split all properties into two groups: Regular properties and reachability checks
    let (properties_without_reachs, reach_checks) = filter_reach_checks(properties_with_undefined);
    // println!("properties_without_reachs: {:?}\n", properties_without_reachs);
    // println!("reach_checks: {:?}\n", reach_checks);
    // Filter out successful sanity checks introduced during compilation
    let properties_without_sanity_checks = filter_sanity_checks(properties_without_reachs);
    // println!("properties_without_sanity_checks: {:?}\n", properties_without_sanity_checks);
    // Annotate properties with the results from reachability checks
    let properties_annotated =
        annotate_properties_with_reach_results(properties_without_sanity_checks, reach_checks);
    // println!("properties_annotated: {:?}\n", properties_annotated);
    // Remove reachability check IDs from regular property descriptions
    let properties_without_ids = remove_check_ids_from_description(properties_annotated);
    // println!("properties_without_ids: {:?}\n", properties_without_ids);

    // Filter out extra pointer checks if needed
    let new_properties = if !extra_ptr_checks {
        filter_ptr_checks(properties_without_ids)
    } else {
        properties_without_ids
    };
    let has_fundamental_failures = has_reachable_unsupported_constructs
        || has_failed_unwinding_asserts
        || has_reachable_undefined_functions;
    // Update the status of properties according to reachability checks, among other things
    let updated_properties =
        update_properties_with_reach_status(new_properties, has_fundamental_failures);

    let overall_result = determine_verification_result(&updated_properties);
    (updated_properties, overall_result)
}

/// Determines if there is property with status `FAILURE` and the given description
fn has_check_failure(properties: &Vec<Property>, description: &str) -> bool {
    for prop in properties {
        if prop.status == CheckStatus::Failure && prop.description.contains(description) {
            return true;
        }
    }
    false
}

/// Replaces the description of all properties from functions with a missing
/// definition.
/// TODO: This hasn't been working as expected, see
/// <https://github.com/model-checking/kani/issues/1424>
fn modify_undefined_function_checks(mut properties: Vec<Property>) -> (Vec<Property>, bool) {
    let mut has_unknown_location_checks = false;
    for mut prop in &mut properties {
        if prop.description.contains(ASSERTION_FALSE)
            && extract_property_class(&prop).unwrap() == DEFAULT_ASSERTION
            && prop.source_location.file.is_none()
        {
            prop.description = "Function with missing definition is unreachable".to_string();
            if prop.status == CheckStatus::Failure {
                has_unknown_location_checks = true;
            }
        };
    }
    (properties, has_unknown_location_checks)
}

/// Returns a user friendly property description.
///
/// `CBMC_ALT_DESCRIPTIONS` is a hash map where:
///  * The key is a property class.
///  * The value is a vector of pairs. In each of these pairs, the first member
///    is a description used to match (with method `contains`) on the original
///    property. If a match is found, we inspect the second member:
///     * If it's `None`, we replace the original property with the description
///       used to match.
///     * If it's `Some(string)`, we replace the original property with `string`.
///
/// For CBMC checks, this will ensure that check failures do not include any
/// temporary variable in their descriptions.
fn get_readable_description(property: &Property) -> String {
    let original = property.description.clone();
    let class_id = extract_property_class(property).unwrap();

    let description_alternatives = CBMC_ALT_DESCRIPTIONS.get(class_id);
    if description_alternatives.is_some() {
        let alt_descriptions = description_alternatives.unwrap();
        for (desc_to_match, opt_desc_to_replace) in alt_descriptions {
            if original.contains(desc_to_match) {
                if opt_desc_to_replace.is_some() {
                    let desc_to_replace = opt_desc_to_replace.unwrap();
                    return desc_to_replace.to_string();
                } else {
                    return desc_to_match.to_string();
                }
            }
        }
    }
    original
}

/// Performs a last pass to update all properties as follows:
///  1. Descriptions are replaced with more readable ones.
///  2. If there were failures that made the verification result unreliable
///     (e.g., a reachable unsupported construct), changes all `SUCCESS` results
///     to `UNDETERMINED`.
///  3. If there weren't such failures, it updates all results with a `SUCCESS`
///     reachability check to `UNREACHABLE`.
fn update_properties_with_reach_status(
    mut properties: Vec<Property>,
    has_fundamental_failures: bool,
) -> Vec<Property> {
    for prop in properties.iter_mut() {
        prop.description = get_readable_description(&prop);
        if has_fundamental_failures {
            if prop.status == CheckStatus::Success {
                prop.status = CheckStatus::Undetermined;
            }
        } else if prop.reach.is_some() && prop.reach.unwrap() == CheckStatus::Success {
            let description = &prop.description;
            assert!(
                prop.status == CheckStatus::Success,
                "** ERROR: Expecting the unreachable property \"{}\" to have a status of \"SUCCESS\"",
                description
            );
            prop.status = CheckStatus::Unreachable
        }
    }
    properties
}

/// Some Kani-generated asserts have a unique ID in their description of the form:
/// ```
/// [KANI_CHECK_ID_<crate-fn-name>_<index>]
/// ```
/// e.g.:
/// ```
/// [KANI_CHECK_ID_foo.6875c808::foo_0] assertion failed: x % 2 == 0
/// ```
/// This function removes those IDs from the property's description so that
/// they're not shown to the user. The removal of the IDs should only be done
/// after all ID-based post-processing is done.
fn remove_check_ids_from_description(mut properties: Vec<Property>) -> Vec<Property> {
    let check_id_pat = Regex::new(r"\[KANI_CHECK_ID_([^\]]*)\] ").unwrap();
    for prop in properties.iter_mut() {
        prop.description = check_id_pat.replace(prop.description.as_str(), "").to_string();
    }
    properties
}

/// Extracts the property class from the property string.
///
/// Property strings have the format `([<function>.]<property_class_id>.<counter>)`
fn extract_property_class(property: &Property) -> Option<&str> {
    let property_class: Vec<&str> = property.property.rsplitn(3, ".").collect();
    if property_class.len() > 1 { Some(property_class[1]) } else { None }
}

/// Given a description, this splits properties into two groups:
///  1. Properties that don't contain the description
///  2. Properties that contain the description
fn filter_properties(properties: Vec<Property>, message: &str) -> (Vec<Property>, Vec<Property>) {
    let mut filtered_properties = Vec::<Property>::new();
    let mut removed_properties = Vec::<Property>::new();
    for prop in properties {
        if prop.description.contains(message) {
            removed_properties.push(prop);
        } else {
            filtered_properties.push(prop);
        }
    }
    (filtered_properties, removed_properties)
}

/// Filters reachability checks with `filter_properties`
fn filter_reach_checks(properties: Vec<Property>) -> (Vec<Property>, Vec<Property>) {
    filter_properties(properties, REACH_CHECK_DESC)
}

/// Filters out Kani-generated sanity checks with a `SUCCESS` status
fn filter_sanity_checks(properties: Vec<Property>) -> Vec<Property> {
    properties
        .into_iter()
        .filter(|prop| {
            !(extract_property_class(prop).unwrap() == "sanity_check"
                && prop.status == CheckStatus::Success)
        })
        .collect()
}

/// Filters out properties related to extra pointer checks
///
/// Our support for primitives and overflow pointer checks is unstable and
/// can result in lots of spurious failures. By default, we filter them out.
fn filter_ptr_checks(properties: Vec<Property>) -> Vec<Property> {
    let props = properties
        .into_iter()
        .filter(|prop| {
            !extract_property_class(prop).unwrap().contains("pointer_arithmetic")
                && !extract_property_class(prop).unwrap().contains("pointer_primitives")
        })
        .collect();
    props
}

/// When assertion reachability checks are turned on, Kani prefixes each
/// assert's description with an ID of the following form:
/// ```
/// [KANI_CHECK_ID_<crate-name>_<index-of-check>]
/// ```
/// e.g.:
/// ```
/// [KANI_CHECK_ID_foo.6875c808::foo_0] assertion failed: x % 2 == 0
/// ```
/// In addition, the description of each reachability check that it generates
/// includes the ID of the assert for which we want to check its reachability.
/// The description of a reachability check uses the following template:
/// ```
/// [KANI_REACHABILITY_CHECK] <ID of original assert>
/// ```
/// e.g.:
/// ```
/// [KANI_REACHABILITY_CHECK] KANI_CHECK_ID_foo.6875c808::foo_0
/// ```
/// This function first collects all data from reachability checks. Then,
/// it updates the reachability status for all properties accordingly.
fn annotate_properties_with_reach_results(
    mut properties: Vec<Property>,
    reach_checks: Vec<Property>,
) -> Vec<Property> {
    let mut reach_map: HashMap<String, CheckStatus> = HashMap::new();
    let reach_desc_pat = Regex::new("KANI_CHECK_ID_.*_([0-9])*").unwrap();
    // Collect data (ID, status) from reachability checks
    for reach_check in reach_checks {
        let description = reach_check.description;
        // Capture the ID in the reachability check
        let check_id =
            reach_desc_pat.captures(description.as_str()).unwrap().get(0).unwrap().as_str();
        let check_id_str = format!("[{}]", check_id);
        // Get the status and insert into `reach_map`
        let status = reach_check.status;
        let res_ins = reach_map.insert(check_id_str, status);
        assert!(res_ins.is_none());
    }

    for prop in properties.iter_mut() {
        let description = &prop.description;
        let check_marker_pat = Regex::new(r"\[KANI_CHECK_ID_([^\]]*)\]").unwrap();
        if check_marker_pat.is_match(description) {
            // Capture the ID in the property
            let prop_match_id =
                check_marker_pat.captures(description.as_str()).unwrap().get(0).unwrap().as_str();
            // Get the status associated to the ID we captured
            let reach_status = reach_map.get(&prop_match_id.to_string());
            // Update the reachability status of the property
            if reach_status.is_some() {
                prop.reach = Some(*reach_status.unwrap());
            }
        }
    }
    properties
}

/// Gets the overall verification result (i.e., failure if any properties show failure)
fn determine_verification_result(properties: &Vec<Property>) -> bool {
    let number_failed_properties =
        properties.iter().filter(|prop| prop.status == CheckStatus::Failure).count();
    number_failed_properties == 0
}
