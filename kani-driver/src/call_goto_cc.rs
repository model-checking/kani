// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::KaniSession;

impl KaniSession {
    /// Given a set of goto binaries (`inputs`), produce `output` by linking everything
    /// together (including essential libraries). The result is generic over all proof harnesses.
    pub fn link_goto_binary(&self, inputs: &[PathBuf], output: &Path) -> Result<()> {
        let mut args: Vec<OsString> = Vec::new();
        args.extend(inputs.iter().map(|x| x.clone().into_os_string()));
        args.extend(self.args.c_lib.iter().map(|x| x.clone().into_os_string()));

        // TODO think about this: kani_lib_c is just an empty c file. Maybe we could just
        // create such an empty file ourselves instead of having to look up this path.
        args.push(self.kani_lib_c.clone().into_os_string());

        args.push("-o".into());
        args.push(output.to_owned().into_os_string());

        // TODO get goto-cc path from self
        let mut cmd = Command::new("goto-cc");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(())
    }

    /// Produce a goto binary with its entry point set to a particular proof harness.
    pub fn specialize_to_proof_harness(
        &self,
        input: &Path,
        output: &Path,
        function: &str,
    ) -> Result<()> {
        let mut cmd = Command::new("goto-cc");
        cmd.arg(input).args(["--function", function, "-o"]).arg(output);

        self.run_suppress(cmd)?;

        Ok(())
    }
}
