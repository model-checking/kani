// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::session::KaniSession;
use crate::util::alter_extension;

impl KaniSession {
    /// Run CBMC appropriately to produce 3 output XML files, then run cbmc-viewer on them to produce a report.
    pub fn run_visualize(&self, file: &Path, default_reportdir: &str) -> Result<()> {
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

        let reportdir = if let Some(pb) = &self.args.target_dir {
            pb.join("report").into_os_string()
        } else {
            default_reportdir.into()
        };

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
            reportdir.clone(),
        ];

        // TODO get cbmc-viewer path from self
        let mut cmd = Command::new("cbmc-viewer");
        cmd.args(args);

        self.run_suppress(cmd)?;

        // Let the user know
        if !self.args.quiet {
            println!("Report written to: {}/html/index.html", reportdir.to_string_lossy());
        }

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
