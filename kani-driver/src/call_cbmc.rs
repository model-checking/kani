// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use kani_metadata::HarnessMetadata;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use crate::args::KaniArgs;
use crate::cbmc_output_parser::process_cbmc_output;
use crate::session::KaniSession;

#[derive(PartialEq, Eq)]
pub enum VerificationStatus {
    Success,
    Failure,
}

impl KaniSession {
    /// Verify a goto binary that's been prepared with goto-instrument
    pub fn run_cbmc(&self, file: &Path, harness: &HarnessMetadata) -> Result<VerificationStatus> {
        let output_filename = crate::util::append_path(file, "cbmc_output");
        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(output_filename.clone());
        }

        let args: Vec<OsString> = self.cbmc_flags(file, harness)?;

        // TODO get cbmc path from self
        let mut cmd = Command::new("cbmc");
        cmd.args(args);

        let now = Instant::now();

        let verification_result = if self.args.output_format == crate::args::OutputFormat::Old {
            if self.run_terminal(cmd).is_err() {
                Ok(VerificationStatus::Failure)
            } else {
                Ok(VerificationStatus::Success)
            }
        } else {
            // Add extra argument to receive the output in JSON format.
            // Done here because `--visualize` uses the XML format instead.
            cmd.arg("--json-ui");

            // Spawn the CBMC process and process its output below
            let cbmc_process_opt = self.run_piped(cmd)?;
            if let Some(cbmc_process) = cbmc_process_opt {
                // The introduction of reachability checks forces us to decide
                // the verification result based on the postprocessing of CBMC results.
                let output_filename_opt: Option<&Path> =
                    if self.args.gen_conc_playback { Some(&output_filename) } else { None };
                let processed_result = process_cbmc_output(
                    cbmc_process,
                    self.args.extra_pointer_checks,
                    &self.args.output_format,
                    output_filename_opt,
                );
                Ok(processed_result)
            } else {
                Ok(VerificationStatus::Failure)
            }
        };
        // TODO: We should print this even the verification fails but not if it crashes.
        if !self.args.dry_run {
            let elapsed = now.elapsed().as_secs_f32();
            println!("Verification Time: {}s", elapsed);
        }

        if let Ok(VerificationStatus::Failure) = verification_result {
            self.gen_and_add_exe_trace(&output_filename, harness);
        }

        verification_result
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

        if !self.args.visualize && !self.args.gen_conc_playback && !self.args.no_slice_formula {
            args.push("--slice-formula".into());
        }

        if self.args.gen_conc_playback {
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
