// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use kani_metadata::{CbmcSolver, HarnessMetadata};
use std::ffi::OsString;
use std::fmt::Write;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::args::{KaniArgs, OutputFormat};
use crate::cbmc_output_parser::{
    extract_results, process_cbmc_output, CheckStatus, ParserItem, Property, VerificationOutput,
};
use crate::cbmc_property_renderer::{format_result, kani_cbmc_output_filter};
use crate::session::KaniSession;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Success,
    Failure,
}

/// Represents failed properties in three different categories.
/// This simplifies the process to determine and format verification results.
#[derive(Clone, Copy, Debug)]
pub enum FailedProperties {
    // No failures
    None,
    // One or more panic-related failures
    PanicsOnly,
    // One or more failures that aren't panic-related
    Other,
}

/// Our (kani-driver) notions of CBMC results.
#[derive(Debug)]
pub struct VerificationResult {
    /// Whether verification should be considered to have succeeded, or have failed.
    pub status: VerificationStatus,
    /// The compact representation for failed properties
    pub failed_properties: FailedProperties,
    /// The parsed output, message by message, of CBMC. However, the `Result` message has been
    /// removed and is available in `results` instead.
    pub messages: Option<Vec<ParserItem>>,
    /// The `Result` properties in detail or the exit_status of CBMC.
    /// Note: CBMC process exit status is only potentially useful if `status` is `Failure`.
    /// Kani will see CBMC report "failure" that's actually success (interpreting "failed"
    /// checks like coverage as expected and desirable.)
    pub results: Result<Vec<Property>, i32>,
    /// The runtime duration of this CBMC invocation.
    pub runtime: Duration,
    /// Whether concrete playback generated a test
    pub generated_concrete_test: bool,
}

impl KaniSession {
    /// Verify a goto binary that's been prepared with goto-instrument
    pub fn run_cbmc(&self, file: &Path, harness: &HarnessMetadata) -> Result<VerificationResult> {
        let args: Vec<OsString> = self.cbmc_flags(file, harness)?;

        // TODO get cbmc path from self
        let mut cmd = Command::new("cbmc");
        cmd.args(args);

        let start_time = Instant::now();

        let mut verification_results = if self.args.output_format == crate::args::OutputFormat::Old
        {
            if self.run_terminal(cmd).is_err() {
                VerificationResult::mock_failure()
            } else {
                VerificationResult::mock_success()
            }
        } else {
            // Add extra argument to receive the output in JSON format.
            // Done here because `--visualize` uses the XML format instead.
            cmd.arg("--json-ui");

            // Spawn the CBMC process and process its output below
            let cbmc_process_opt = self.run_piped(cmd)?;
            let cbmc_process = cbmc_process_opt.ok_or(anyhow::Error::msg("Failed to run cbmc"))?;
            let output = process_cbmc_output(cbmc_process, |i| {
                kani_cbmc_output_filter(
                    i,
                    self.args.extra_pointer_checks,
                    self.args.quiet,
                    &self.args.output_format,
                )
            })?;

            VerificationResult::from(output, harness.attributes.should_panic, start_time)
        };

        self.gen_and_add_concrete_playback(harness, &mut verification_results)?;
        Ok(verification_results)
    }

    /// used by call_cbmc_viewer, invokes different variants of CBMC.
    // TODO: this could use some cleanup and refactoring.
    pub fn call_cbmc(&self, args: Vec<OsString>, output: &Path) -> Result<()> {
        // TODO get cbmc path from self
        let mut cmd = Command::new("cbmc");
        cmd.args(args);

        let result = self.run_redirect(cmd, output)?;

        if !result.success() {
            bail!("cbmc exited with status {}", result);
        }
        // TODO: We 'bail' above, but then ignore it in 'call_cbmc_viewer' ...

        Ok(())
    }

    /// "Internal," but also used by call_cbmc_viewer
    pub fn cbmc_flags(
        &self,
        file: &Path,
        harness_metadata: &HarnessMetadata,
    ) -> Result<Vec<OsString>> {
        let mut args = self.cbmc_check_flags();

        if let Some(object_bits) = self.args.cbmc_object_bits() {
            args.push("--object-bits".into());
            args.push(object_bits.to_string().into());
        }

        if let Some(unwind_value) = resolve_unwind_value(&self.args, harness_metadata) {
            args.push("--unwind".into());
            args.push(unwind_value.to_string().into());
        }

        self.handle_solver_args(&harness_metadata.attributes.solver, &mut args)?;

        if self.args.run_sanity_checks {
            args.push("--validate-goto-model".into());
            args.push("--validate-ssa-equation".into());
        }

        if !self.args.visualize
            && self.args.concrete_playback.is_none()
            && !self.args.no_slice_formula
        {
            args.push("--slice-formula".into());
        }

        if self.args.concrete_playback.is_some() {
            args.push("--trace".into());
        }

        args.extend(self.args.cbmc_args.iter().cloned());

        args.push(file.to_owned().into_os_string());

        Ok(args)
    }

    /// Just the flags to CBMC that enable property checking of any sort.
    pub fn cbmc_check_flags(&self) -> Vec<OsString> {
        let mut args = Vec::new();

        if self.args.checks.memory_safety_on() {
            args.push("--bounds-check".into());
            args.push("--pointer-check".into());
        }
        if self.args.checks.overflow_on() {
            args.push("--div-by-zero-check".into());
            args.push("--float-overflow-check".into());
            args.push("--nan-check".into());
            args.push("--undefined-shift-check".into());
            // With PR #647 we use Rust's `-C overflow-checks=on` instead of:
            // --unsigned-overflow-check
            // --signed-overflow-check
            // So these options are deliberately skipped to avoid erroneously re-checking operations.

            // TODO: Implement conversion checks as an optional check.
            // They are a well defined operation in rust, but they may yield unexpected results to
            // many users. https://github.com/model-checking/kani/issues/840
            // We might want to create a transformation pass instead of enabling CBMC since Kani
            // compiler sometimes rely on the bitwise conversion of signed <-> unsigned.
            // args.push("--conversion-check".into());
        }

        if self.args.checks.unwinding_on() {
            args.push("--unwinding-assertions".into());
        }

        if self.args.extra_pointer_checks {
            // This was adding a lot of false positives with std dangling pointer. We should
            // still catch any invalid dereference with --pointer-check. Thus, only enable them
            // if the user explicitly request them.
            args.push("--pointer-overflow-check".into());
            args.push("--pointer-primitive-check".into());
        }

        args
    }

    pub fn handle_solver_args(
        &self,
        harness_solver: &Option<CbmcSolver>,
        args: &mut Vec<OsString>,
    ) -> Result<()> {
        let solver = if let Some(solver) = &self.args.solver {
            // `--solver` option takes precedence over attributes
            solver
        } else if let Some(solver) = harness_solver {
            solver
        } else {
            // Nothing to do
            return Ok(());
        };

        match solver {
            CbmcSolver::Cadical => {
                args.push("--sat-solver".into());
                args.push("cadical".into());
            }
            CbmcSolver::Kissat => {
                args.push("--external-sat-solver".into());
                args.push("kissat".into());
            }
            CbmcSolver::Minisat => {
                // Minisat is currently CBMC's default solver, so no need to
                // pass any arguments
            }
            CbmcSolver::Binary(solver_binary) => {
                // Check if the specified binary exists in path
                if which::which(solver_binary).is_err() {
                    bail!("the specified solver \"{solver_binary}\" was not found in path")
                }
                args.push("--external-sat-solver".into());
                args.push(solver_binary.into());
            }
        }
        Ok(())
    }
}

impl VerificationResult {
    /// Computes a `VerificationResult` (kani-driver's notion of the result of a CBMC call) from a
    /// `VerificationOutput` (cbmc_output_parser's idea of CBMC results).
    ///
    /// NOTE: We actually ignore the CBMC exit status, in favor of two checks:
    ///   1. Examining the actual results of CBMC properties.
    ///       (CBMC will regularly report "failure" but that's just our cover checks.)
    ///   2. Positively checking for the presence of results.
    ///       (Do not mistake lack of results for success: report it as failure.)
    fn from(
        output: VerificationOutput,
        should_panic: bool,
        start_time: Instant,
    ) -> VerificationResult {
        let runtime = start_time.elapsed();
        let (items, results) = extract_results(output.processed_items);

        if let Some(results) = results {
            let (status, failed_properties) =
                verification_outcome_from_properties(&results, should_panic);
            VerificationResult {
                status,
                failed_properties,
                messages: Some(items),
                results: Ok(results),
                runtime,
                generated_concrete_test: false,
            }
        } else {
            // We never got results from CBMC - something went wrong (e.g. crash) so it's failure
            VerificationResult {
                status: VerificationStatus::Failure,
                failed_properties: FailedProperties::Other,
                messages: Some(items),
                results: Err(output.process_status),
                runtime,
                generated_concrete_test: false,
            }
        }
    }

    pub fn mock_success() -> VerificationResult {
        VerificationResult {
            status: VerificationStatus::Success,
            failed_properties: FailedProperties::None,
            messages: None,
            results: Ok(vec![]),
            runtime: Duration::from_secs(0),
            generated_concrete_test: false,
        }
    }

    fn mock_failure() -> VerificationResult {
        VerificationResult {
            status: VerificationStatus::Failure,
            failed_properties: FailedProperties::Other,
            messages: None,
            // on failure, exit codes in theory might be used,
            // but `mock_failure` should never be used in a context where they will,
            // so again use something weird:
            results: Err(42),
            runtime: Duration::from_secs(0),
            generated_concrete_test: false,
        }
    }

    pub fn render(&self, output_format: &OutputFormat, should_panic: bool) -> String {
        match &self.results {
            Ok(results) => {
                let status = self.status;
                let failed_properties = self.failed_properties;
                let show_checks = matches!(output_format, OutputFormat::Regular);
                let mut result =
                    format_result(results, status, should_panic, failed_properties, show_checks);
                writeln!(result, "Verification Time: {}s", self.runtime.as_secs_f32()).unwrap();
                result
            }
            Err(exit_status) => {
                let verification_result = console::style("FAILED").red();
                format!(
                    "\nCBMC failed with status {exit_status}\nVERIFICATION:- {verification_result}\n",
                )
            }
        }
    }

    /// Find the failed properties from this verification run
    pub fn failed_properties(&self) -> Vec<&Property> {
        if let Ok(properties) = &self.results {
            properties.iter().filter(|prop| prop.status == CheckStatus::Failure).collect()
        } else {
            debug_assert!(false, "expected error to be handled before invoking this function");
            vec![]
        }
    }
}

/// We decide if verification succeeded based on properties, not (typically) on exit code
fn verification_outcome_from_properties(
    properties: &[Property],
    should_panic: bool,
) -> (VerificationStatus, FailedProperties) {
    let failed_properties = determine_failed_properties(properties);
    let status = if should_panic {
        match failed_properties {
            FailedProperties::None | FailedProperties::Other => VerificationStatus::Failure,
            FailedProperties::PanicsOnly => VerificationStatus::Success,
        }
    } else {
        match failed_properties {
            FailedProperties::None => VerificationStatus::Success,
            FailedProperties::PanicsOnly | FailedProperties::Other => VerificationStatus::Failure,
        }
    };
    (status, failed_properties)
}

/// Determines the `FailedProperties` variant that corresponds to an array of properties
fn determine_failed_properties(properties: &[Property]) -> FailedProperties {
    let failed_properties: Vec<&Property> =
        properties.iter().filter(|prop| prop.status == CheckStatus::Failure).collect();
    // Return `FAILURE` if there isn't at least one failed property
    if failed_properties.is_empty() {
        FailedProperties::None
    } else {
        // Check if all failed properties correspond to the `assertion` class.
        // Note: Panics caused by `panic!` and `assert!` fall into this class.
        let all_failed_checks_are_panics =
            failed_properties.iter().all(|prop| prop.property_class() == "assertion");
        if all_failed_checks_are_panics {
            FailedProperties::PanicsOnly
        } else {
            FailedProperties::Other
        }
    }
}

/// Solve Unwind Value from conflicting inputs of unwind values. (--default-unwind, annotation-unwind, --unwind)
pub fn resolve_unwind_value(args: &KaniArgs, harness_metadata: &HarnessMetadata) -> Option<u32> {
    // Check for which flag is being passed and prioritize extracting unwind from the
    // respective flag/annotation.
    args.unwind.or(harness_metadata.attributes.unwind_value).or(args.default_unwind)
}

#[cfg(test)]
mod tests {
    use crate::args;
    use crate::metadata::mock_proof_harness;
    use clap::Parser;

    use super::*;

    #[test]
    fn check_resolve_unwind_value() {
        // Command line unwind value for specific harnesses take precedence over default annotation value
        let args_empty = ["kani", "x.rs"];
        let args_only_default = ["kani", "x.rs", "--default-unwind", "2"];
        let args_only_harness = ["kani", "x.rs", "--unwind", "1", "--harness", "check_one"];
        let args_both =
            ["kani", "x.rs", "--default-unwind", "2", "--unwind", "1", "--harness", "check_one"];

        let harness_none = mock_proof_harness("check_one", None, None, None);
        let harness_some = mock_proof_harness("check_one", Some(3), None, None);

        fn resolve(args: &[&str], harness: &HarnessMetadata) -> Option<u32> {
            resolve_unwind_value(
                &args::StandaloneArgs::try_parse_from(args).unwrap().common_opts,
                harness,
            )
        }

        // test against no unwind annotation
        assert_eq!(resolve(&args_empty, &harness_none), None);
        assert_eq!(resolve(&args_only_default, &harness_none), Some(2));
        assert_eq!(resolve(&args_only_harness, &harness_none), Some(1));
        assert_eq!(resolve(&args_both, &harness_none), Some(1));

        // test against unwind annotation
        assert_eq!(resolve(&args_empty, &harness_some), Some(3));
        assert_eq!(resolve(&args_only_default, &harness_some), Some(3));
        assert_eq!(resolve(&args_only_harness, &harness_some), Some(1));
        assert_eq!(resolve(&args_both, &harness_some), Some(1));
    }
}
