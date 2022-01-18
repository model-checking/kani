// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::context::KaniContext;

impl KaniContext {
    /// Verify a goto binary that's been prepared with goto-instrument
    pub fn format_cbmc_output(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![self.cbmc_json_parser_py.clone().into(), file.into()];

        let result = Command::new("python")
            .args(args)
            .status()
            .context("Failed to invoke cbmc_json_parser.py")?;

        if !result.success() {
            bail!("cbmc_json_parser.py exited with status {}", result);
        }

        Ok(())
    }
}
