// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use kani_metadata::HarnessMetadata;
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

#[derive(Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Success,
    Failure,
}

/// Our (kani-driver) notions of CBMC results.
#[derive(Debug)]
pub struct VerificationResult {
    /// Whether verification should be considered to have succeeded, or have failed.
    pub status: VerificationStatus,
    /// The parsed output, message by message, of CBMC. However, the `Result` message has been
    /// removed and is available in `results` instead.
    pub messages: Option<Vec<ParserItem>>,
    /// The `Result` properties in detail.
    pub results: Option<Vec<Property>>,
    /// CBMC process exit status. NOTE: Only potentially useful if `status` is `Failure`.
    /// Kani will see CBMC report "failure" that's actually success (interpreting "failed"
    /// checks like coverage as expected and desirable.)
    pub exit_status: i32,
    /// The runtime duration of this CBMC invocation.
    pub runtime: Duration,
}

impl KaniSession {
    /// Verify a goto binary that's been prepared with goto-instrument
    pub fn run_cbmc(&self, file: &Path, harness: &HarnessMetadata) -> Result<VerificationResult> {
        let args: Vec<OsString> = self.cbmc_flags(file, harness)?;

        // TODO get cbmc path from self
        let mut cmd = Command::new("cbmc");
        cmd.args(args);

        let start_time = Instant::now();

        let verification_results = if self.args.output_format == crate::args::OutputFormat::Old {
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
            if let Some(cbmc_process) = cbmc_process_opt {
                let output = process_cbmc_output(cbmc_process, |i| {
                    kani_cbmc_output_filter(
                        i,
                        self.args.extra_pointer_checks,
                        &self.args.output_format,
                    )
                })?;

                VerificationResult::from(output, start_time)
            } else {
                // None is only ever returned when it's a dry run
                VerificationResult::mock_success()
            }
        };

        self.gen_and_add_concrete_playback(harness, &verification_results)?;
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
    fn from(output: VerificationOutput, start_time: Instant) -> VerificationResult {
        let runtime = start_time.elapsed();
        let (items, results) = extract_results(output.processed_items);

        if let Some(results) = results {
            VerificationResult {
                status: determine_status_from_properties(&results),
                messages: Some(items),
                results: Some(results),
                exit_status: output.process_status,
                runtime,
            }
        } else {
            // We never got results from CBMC - something went wrong (e.g. crash) so it's failure
            VerificationResult {
                status: VerificationStatus::Failure,
                messages: Some(items),
                results: None,
                exit_status: output.process_status,
                runtime,
            }
        }
    }

    pub fn mock_success() -> VerificationResult {
        VerificationResult {
            status: VerificationStatus::Success,
            messages: None,
            results: None,
            exit_status: 42, // on success, exit code is ignored, so put something weird here
            runtime: Duration::from_secs(0),
        }
    }

    fn mock_failure() -> VerificationResult {
        VerificationResult {
            status: VerificationStatus::Failure,
            messages: None,
            results: None,
            // on failure, exit codes in theory might be used,
            // but `mock_failure` should never be used in a context where they will,
            // so again use something weird:
            exit_status: 42,
            runtime: Duration::from_secs(0),
        }
    }

    pub fn render(&self, output_format: &OutputFormat) -> String {
        if let Some(results) = &self.results {
            let show_checks = matches!(output_format, OutputFormat::Regular);
            let mut result = format_result(results, show_checks);
            writeln!(result, "Verification Time: {}s", self.runtime.as_secs_f32()).unwrap();
            result
        } else {
            let verification_result = console::style("FAILED").red();
            format!(
                "\nCBMC failed with status {}\nVERIFICATION:- {}\n",
                self.exit_status, verification_result
            )
        }
    }
}

/// We decide if verificaiton succeeded based on properties, not (typically) on exit code
fn determine_status_from_properties(properties: &[Property]) -> VerificationStatus {
    let number_failed_properties =
        properties.iter().filter(|prop| prop.status == CheckStatus::Failure).count();
    if number_failed_properties == 0 {
        VerificationStatus::Success
    } else {
        VerificationStatus::Failure
    }
}

/// Solve Unwind Value from conflicting inputs of unwind values. (--default-unwind, annotation-unwind, --unwind)
pub fn resolve_unwind_value(args: &KaniArgs, harness_metadata: &HarnessMetadata) -> Option<u32> {
    // Check for which flag is being passed and prioritize extracting unwind from the
    // respective flag/annotation.
    args.unwind.or(harness_metadata.unwind_value).or(args.default_unwind)
}

#[cfg(test)]
mod tests {
    use crate::args;
    use crate::metadata::mock_proof_harness;
    use structopt::StructOpt;

    use super::*;

    #[test]
    fn check_resolve_unwind_value() {
        // Command line unwind value for specific harnesses take precedence over default annotation value
        let args_empty = ["kani"];
        let args_only_default = ["kani", "--default-unwind", "2"];
        let args_only_harness = ["kani", "--unwind", "1", "--harness", "check_one"];
        let args_both =
            ["kani", "--default-unwind", "2", "--unwind", "1", "--harness", "check_one"];

        let harness_none = mock_proof_harness("check_one", None);
        let harness_some = mock_proof_harness("check_one", Some(3));

        // test against no unwind annotation
        assert_eq!(
            resolve_unwind_value(&args::KaniArgs::from_iter(args_empty), &harness_none),
            None
        );
        assert_eq!(
            resolve_unwind_value(&args::KaniArgs::from_iter(args_only_default), &harness_none),
            Some(2)
        );
        assert_eq!(
            resolve_unwind_value(&args::KaniArgs::from_iter(args_only_harness), &harness_none),
            Some(1)
        );
        assert_eq!(
            resolve_unwind_value(&args::KaniArgs::from_iter(args_both), &harness_none),
            Some(1)
        );

        // test against unwind annotation
        assert_eq!(
            resolve_unwind_value(&args::KaniArgs::from_iter(args_empty), &harness_some),
            Some(3)
        );
        assert_eq!(
            resolve_unwind_value(&args::KaniArgs::from_iter(args_only_default), &harness_some),
            Some(3)
        );
        assert_eq!(
            resolve_unwind_value(&args::KaniArgs::from_iter(args_only_harness), &harness_some),
            Some(1)
        );
        assert_eq!(
            resolve_unwind_value(&args::KaniArgs::from_iter(args_both), &harness_some),
            Some(1)
        );
    }
}
