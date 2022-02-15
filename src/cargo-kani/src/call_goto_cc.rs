// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::args::AbstractionType;
use crate::context::KaniContext;

impl KaniContext {
    /// Given a set of goto binaries (`inputs`), produce `output` by linking everything
    /// together (including essential libraries) and also specializing to the proof harness
    /// `function`.
    pub fn link_c_lib(&self, inputs: &[PathBuf], output: &Path, function: &str) -> Result<()> {
        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(output.to_owned());
        }

        let mut args: Vec<OsString> = vec!["--function".into(), function.into()];
        args.extend(inputs.iter().map(|x| x.clone().into_os_string()));
        args.extend(self.args.c_lib.iter().map(|x| x.clone().into_os_string()));

        // Special case hack for handling the "c-ffi" abs-type
        if self.args.use_abs && self.args.abs_type == AbstractionType::CFfi {
            let mut vec = self.kani_c_stubs.clone();
            vec.push("vec");
            vec.push("vec.c");
            let mut hashset = self.kani_c_stubs.clone();
            hashset.push("hashset");
            hashset.push("hashset.c");

            args.push(vec.into_os_string());
            args.push(hashset.into_os_string());
        }

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
}
