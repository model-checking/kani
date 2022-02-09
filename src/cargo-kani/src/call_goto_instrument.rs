// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::context::KaniContext;
use crate::util::alter_extension;

impl KaniContext {
    /// Postprocess a goto binary (before cbmc) in-place by calling goto-instrument
    pub fn run_goto_instrument(&self, file: &Path) -> Result<()> {
        if self.args.checks.undefined_function_on() {
            self.add_library(file)?;
            self.undefined_functions(file)?;
        } else {
            self.just_drop_unused_functions(file)?;
        }

        if self.args.gen_c {
            self.gen_c(file)?;
        }

        Ok(())
    }

    /// Apply --restrict-vtable to a goto binary.
    /// `source` is either a `*.restrictions.json` file or a directory containing mutiple such files.
    pub fn apply_vtable_restrictions(&self, file: &Path, source: &Path) -> Result<()> {
        let linked_restrictions = alter_extension(file, "linked-restrictions.json");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(linked_restrictions.clone());
        }

        {
            let mut cmd = Command::new(&self.kani_link_restrictions);
            cmd.args(vec![source.as_os_str(), linked_restrictions.as_os_str()]);

            self.run_suppress(cmd)?;
        }

        let args: Vec<OsString> = vec![
            "--function-pointer-restrictions-file".into(),
            linked_restrictions.into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        // TODO get goto-instrument path from self
        let mut cmd = Command::new("goto-instrument");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(())
    }

    fn add_library(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--add-library".into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        // TODO get goto-instrument path from self
        let mut cmd = Command::new("goto-instrument");
        cmd.args(args);

        self.run_suppress(cmd)?;

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
        let mut cmd = Command::new("goto-instrument");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(())
    }

    fn just_drop_unused_functions(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--drop-unused-functions".into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        // TODO get goto-instrument path from self
        let mut cmd = Command::new("goto-instrument");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(())
    }

    /// Generate a .c file from a goto binary (i.e. --gen-c)
    pub fn gen_c(&self, file: &Path) -> Result<()> {
        let output_filename = alter_extension(file, "c");
        // We don't put the C file into temporaries to be deleted.

        let args: Vec<OsString> = vec![
            "--dump-c".into(),
            file.to_owned().into_os_string(),
            output_filename.into_os_string(),
        ];

        // TODO get goto-instrument path from self
        let mut cmd = Command::new("goto-instrument");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(())
    }
}
