// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use kani_metadata::HarnessMetadata;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::args::KaniArgs;
use crate::session::KaniSession;

#[derive(PartialEq)]
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

        if self.args.output_format == crate::args::OutputFormat::Old {
            let result = self.run_terminal(cmd);
            if !self.args.allow_cbmc_verification_failure && result.is_err() {
                return Ok(VerificationStatus::Failure);
            }
        } else {
            // extra argument
            cmd.arg("--json-ui");

            let _cbmc_result = self.run_redirect(cmd, &output_filename)?;
            let format_result = self.format_cbmc_output(&output_filename);

            if !self.args.allow_cbmc_verification_failure && format_result.is_err() {
                // Because of things like --assertion-reach-checks and other future features,
                // we now decide if we fail or not based solely on the output of the formatter.
                return Ok(VerificationStatus::Failure);
                // todo: this is imperfect, since we don't know why failure happened.
                // the best possible fix is port to rust instead of using python, or getting more
                // feedback than just exit status (or using a particular magic exit code?)
            }
        }

        Ok(VerificationStatus::Success)
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

        args.push("--object-bits".into());
        args.push(self.args.object_bits.to_string().into());

        if let Some(unwind_value) = resolve_unwind_value(&self.args, harness_metadata) {
            args.push("--unwind".into());
            args.push(unwind_value.to_string().into());
        } else if self.args.auto_unwind {
            args.push("--auto-unwind".into());
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
            args.push("--pointer-primitive-check".into());
        }
        if self.args.checks.overflow_on() {
            args.push("--conversion-check".into());
            args.push("--div-by-zero-check".into());
            args.push("--float-overflow-check".into());
            args.push("--nan-check".into());
            args.push("--pointer-overflow-check".into());
            args.push("--undefined-shift-check".into());
            // With PR #647 we use Rust's `-C overflow-checks=on` instead of:
            // --unsigned-overflow-check
            // --signed-overflow-check
            // So these options are deliberately skipped to avoid erroneously re-checking operations.
        }
        if self.args.checks.unwinding_on() {
            args.push("--unwinding-assertions".into());
        }

        args
    }
}

/// Solve Unwind Value from conflicting inputs of unwind values. (--default-unwind, annotation-unwind, --harness-unwind)
pub fn resolve_unwind_value(args: &KaniArgs, harness_metadata: &HarnessMetadata) -> Option<u32> {
    // Check for which flag is being passed and prioritize extracting unwind from the
    // respective flag/annotation.
    if let Some(harness_unwind) = args.harness_unwind {
        Some(harness_unwind)
    } else if let Some(annotation_unwind) = harness_metadata.unwind_value {
        Some(annotation_unwind)
    } else if let Some(default_unwind) = args.default_unwind {
        Some(default_unwind)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::args;
    use crate::metadata::{find_proof_harness, mock_proof_harness};
    use structopt::StructOpt;

    use super::*;

    #[test]
    fn check_default_annotation_unwind_resolve() {
        // Annotation value for unwind takes precedence over default-unwind value
        // except when there is no #[kani::unwind()] provided.
        let args: Vec<OsString> = ["kani", "--no-default-checks", "--default-unwind", "2"]
            .iter()
            .map(|&s| s.into())
            .collect();
        let unwind_args = args::KaniArgs::from_iter(args);

        let harnesses = vec![
            mock_proof_harness("check_one", Some(1)),
            mock_proof_harness("module::check_two", Some(3)),
            mock_proof_harness("module::not_check_three", None),
        ];

        let merged_1 = resolve_unwind_value(
            &unwind_args,
            find_proof_harness("check_one", &harnesses).unwrap(),
        );
        let merged_2 = resolve_unwind_value(
            &unwind_args,
            find_proof_harness("check_two", &harnesses).unwrap(),
        );
        let merged_3 = resolve_unwind_value(
            &unwind_args,
            find_proof_harness("not_check_three", &harnesses).unwrap(),
        );

        assert!(merged_1 == Some(1));
        assert!(merged_2 == Some(3));
        assert!(merged_3 == Some(2));
    }

    #[test]
    fn check_harness_annotation_unwind_resolve() {
        // Command line unwind value for specific harnesses take precedence over unwind annotation value
        let args_1: Vec<OsString> =
            ["kani", "--no-default-checks", "--harness-unwind", "2", "--harness", "check_one"]
                .iter()
                .map(|&s| s.into())
                .collect();
        let args_2: Vec<OsString> =
            ["kani", "--no-default-checks", "--harness-unwind", "2", "--harness", "check_two"]
                .iter()
                .map(|&s| s.into())
                .collect();
        let args_3: Vec<OsString> = [
            "kani",
            "--no-default-checks",
            "--harness-unwind",
            "2",
            "--harness",
            "not_check_three",
        ]
        .iter()
        .map(|&s| s.into())
        .collect();

        let unwind_args_1 = args::KaniArgs::from_iter(args_1);
        let unwind_args_2 = args::KaniArgs::from_iter(args_2);
        let unwind_args_3 = args::KaniArgs::from_iter(args_3);

        let harnesses = vec![
            mock_proof_harness("check_one", Some(1)),
            mock_proof_harness("module::check_two", Some(3)),
            mock_proof_harness("module::not_check_three", None),
        ];

        let merged_1 = resolve_unwind_value(
            &unwind_args_1,
            find_proof_harness("check_one", &harnesses).unwrap(),
        );
        let merged_2 = resolve_unwind_value(
            &unwind_args_2,
            find_proof_harness("check_two", &harnesses).unwrap(),
        );
        let merged_3 = resolve_unwind_value(
            &unwind_args_3,
            find_proof_harness("not_check_three", &harnesses).unwrap(),
        );

        assert!(merged_1 == Some(2));
        assert!(merged_2 == Some(2));
        assert!(merged_3 == Some(2));
    }

    #[test]
    fn check_default_harness_unwind_resolve() {
        // Command line unwind value for specific harnesses take precedence over default annotation value
        let args_1: Vec<OsString> = [
            "kani",
            "--no-default-checks",
            "--default-unwind",
            "2",
            "--harness-unwind",
            "1",
            "--harness",
            "check_one",
        ]
        .iter()
        .map(|&s| s.into())
        .collect();
        let args_2: Vec<OsString> = [
            "kani",
            "--no-default-checks",
            "--default-unwind",
            "2",
            "--harness-unwind",
            "3",
            "--harness",
            "check_two",
        ]
        .iter()
        .map(|&s| s.into())
        .collect();
        let args_3: Vec<OsString> = [
            "kani",
            "--no-default-checks",
            "--default-unwind",
            "2",
            "--harness-unwind",
            "4",
            "--harness",
            "check_three",
        ]
        .iter()
        .map(|&s| s.into())
        .collect();
        let args_4: Vec<OsString> = ["kani", "--no-default-checks", "--default-unwind", "2"]
            .iter()
            .map(|&s| s.into())
            .collect();

        let unwind_args_1 = args::KaniArgs::from_iter(args_1);
        let unwind_args_2 = args::KaniArgs::from_iter(args_2);
        let unwind_args_3 = args::KaniArgs::from_iter(args_3);
        let unwind_args_4 = args::KaniArgs::from_iter(args_4);

        let harnesses = vec![
            mock_proof_harness("check_one", None),
            mock_proof_harness("module::check_two", None),
            mock_proof_harness("module::check_three", None),
            mock_proof_harness("module::not_check_four", None),
        ];

        let merged_1 = resolve_unwind_value(
            &unwind_args_1,
            find_proof_harness("check_one", &harnesses).unwrap(),
        );
        let merged_2 = resolve_unwind_value(
            &unwind_args_2,
            find_proof_harness("check_two", &harnesses).unwrap(),
        );
        let merged_3 = resolve_unwind_value(
            &unwind_args_3,
            find_proof_harness("check_three", &harnesses).unwrap(),
        );
        let merged_4 = resolve_unwind_value(
            &unwind_args_4,
            find_proof_harness("not_check_four", &harnesses).unwrap(),
        );

        assert!(merged_1 == Some(1));
        assert!(merged_2 == Some(3));
        assert!(merged_3 == Some(4));
        assert!(merged_4 == Some(2));
    }
}
