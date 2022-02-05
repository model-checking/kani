// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::context::KaniContext;

impl KaniContext {
    /// Verify a goto binary that's been prepared with goto-instrument
    pub fn run_cbmc(&self, file: &Path) -> Result<PathBuf> {
        let output_filename = crate::util::append_path(file, "cbmc_output");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(output_filename.clone());
        }

        let args: Vec<OsString> = self.cbmc_flags(file)?;

        // TODO get cbmc path from self
        let mut cmd = Command::new("cbmc");
        cmd.args(args);

        let result = self.run_redirect(cmd, &output_filename)?;

        // regardless of success or failure, first we need to print:
        self.format_cbmc_output(&output_filename)?;

        if !result.success() && !self.args.allow_cbmc_verification_failure {
            bail!("cbmc exited with status {}", result);
        }

        Ok(output_filename)
    }

    /// used by call_cbmc_viewer, needs refactor TODO
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
    pub fn cbmc_flags(&self, file: &Path) -> Result<Vec<OsString>> {
        let mut args = self.cbmc_check_flags();

        args.push("--object-bits".into());
        args.push("16".into());
        args.push("--json-ui".into()); // todo unconditional, we always redirect output
        // but todo: we're appending --xml-ui for viewer, which works because it seems to override, but that's unclean
        args.push(file.to_owned().into_os_string());

        Ok(args)
    }

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
        }
        if self.args.checks.unwinding_on() {
            args.push("--unwinding-assertions".into());
        }

        args
    }
}
