// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::context::KaniContext;

impl KaniContext {
    /// Given a `file` (a .symtab.json), produce `{file}.out` by calling symtab2gb
    pub fn link_c_lib(&self, inputs: &[PathBuf], output: &Path, function: &str) -> Result<()> {
        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(output.to_owned());
        }

        let mut args: Vec<OsString> = vec!["--function".into(), function.into()];
        args.extend(inputs.iter().map(|x| x.clone().into_os_string()));

        // TODO think about this
        args.push(self.kani_lib_c.clone().into_os_string());

        args.push("-o".into());
        args.push(output.to_owned().into_os_string());

        // TODO get goto-cc path from self
        let mut cmd = Command::new("goto-cc");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(())
    }
}
