// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::context::KaniContext;
use crate::util::alter_extension;

impl KaniContext {
    /// Verify a goto binary that's been prepared with goto-instrument
    pub fn run_visualize(&self, file: &Path) -> Result<()> {
        let results_filename = alter_extension(file, "results.xml");
        let coverage_filename = alter_extension(file, "coverage.xml");
        let property_filename = alter_extension(file, "property.xml");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(results_filename.clone());
            temps.push(coverage_filename.clone());
            temps.push(property_filename.clone());
        }

        self.cbmc_variant(file, &["--xml-ui", "--trace"], &results_filename)?;
        self.cbmc_variant(file, &["--xml-ui", "--cover", "location"], &coverage_filename)?;
        self.cbmc_variant(file, &["--xml-ui", "--show-properties"], &property_filename)?;

        let args: Vec<OsString> = vec![
            "--result".into(),
            results_filename.into(),
            "--coverage".into(),
            coverage_filename.into(),
            "--property".into(),
            property_filename.into(),
            "--srcdir".into(),
            ".".into(), // os.path.realpath(srcdir),
            "--wkdir".into(),
            ".".into(), // os.path.realpath(wkdir),
            "--goto".into(),
            file.into(),
            "--reportdir".into(),
            "report".into(), //reportdir,
        ];

        // TODO get cbmc-viewer path from self
        let mut cmd = Command::new("cbmc-viewer");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(())
    }

    fn cbmc_variant(&self, file: &Path, extra_args: &[&str], output: &Path) -> Result<()> {
        let mut args = self.cbmc_flags(file)?;
        args.extend(extra_args.iter().map(|x| x.into()));

        // TODO fix this hack, abstractions are wrong
        if extra_args.contains(&"--cover") {
            if let Some(i) = args.iter().position(|x| x == "--unwinding-assertions") {
                args.remove(i);
            }
        }

        // Expect and allow failures... maybe we should do better here somehow
        let _result = self.call_cbmc(args, output);

        Ok(())
    }
}
