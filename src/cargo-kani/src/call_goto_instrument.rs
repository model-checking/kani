// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::context::KaniContext;

impl KaniContext {
    /// Postprocess a goto binary (before cbmc) in-place by calling goto-instrument
    pub fn run_goto_instrument(&self, file: &Path) -> Result<()> {
        self.add_library(file)?;
        self.undefined_functions(file)?;

        Ok(())
    }

    fn add_library(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--add-library".into(),
            file.to_owned().into_os_string(),
            file.to_owned().into_os_string(),
        ];
        // args.push();
        // args.push(); // input
        // args.push(); // output

        // TODO get goto-instrument path from self
        let result = Command::new("goto-instrument")
            .args(args)
            .status()
            .context("Failed to invoke goto-instrument")?;

        if !result.success() {
            bail!("goto-instrument exited with status {}", result);
        }

        Ok(())
    }

    fn undefined_functions(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--generate-function-body-options".into(),
            "assert-false".into(),
            "--generate-function-body".into(),
            ".*".into(),
            "--drop-unused-functions".into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        // TODO get goto-instrument path from self
        let result = Command::new("goto-instrument")
            .args(args)
            .status()
            .context("Failed to invoke goto-instrument")?;

        if !result.success() {
            bail!("goto-instrument exited with status {}", result);
        }

        Ok(())
    }
}
