// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use kani_metadata::HarnessMetadata;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::session::KaniSession;
use crate::util::alter_extension;

impl KaniSession {
    /// Run CBMC appropriately to produce 3 output XML files, then run cbmc-viewer on them to produce a report.
    /// Viewer doesn't give different error codes depending on verification failure, so as long as it works, we report success.
    pub fn run_visualize(
        &self,
        file: &Path,
        report_dir: &Path,
        harness_metadata: &HarnessMetadata,
    ) -> Result<()> {
        let results_filename = alter_extension(file, "results.xml");
        let coverage_filename = alter_extension(file, "coverage.xml");
        let property_filename = alter_extension(file, "property.xml");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(results_filename.clone());
            temps.push(coverage_filename.clone());
            temps.push(property_filename.clone());
        }

        self.cbmc_variant(file, &["--xml-ui", "--trace"], &results_filename, harness_metadata)?;
        self.cbmc_variant(
            file,
            &["--xml-ui", "--cover", "location"],
            &coverage_filename,
            harness_metadata,
        )?;
        self.cbmc_variant(
            file,
            &["--xml-ui", "--show-properties"],
            &property_filename,
            harness_metadata,
        )?;

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
            report_dir.into(),
        ];

        // TODO get cbmc-viewer path from self
        let mut cmd = Command::new("cbmc-viewer");
        cmd.args(args);

        self.run_suppress(cmd)?;

        // Let the user know
        if !self.args.quiet {
            println!("Report written to: {}/html/index.html", report_dir.to_string_lossy());
            // If using VS Code with Remote-SSH, suggest an option for remote viewing:
            if std::env::var("VSCODE_IPC_HOOK_CLI").is_ok()
                && std::env::var("SSH_CONNECTION").is_ok()
            {
                println!(
                    "VSCode Remote-SSH port forwards for you. Try:  python3 -m http.server --directory {}/html",
                    report_dir.to_string_lossy()
                );
            }
        }

        Ok(())
    }

    fn cbmc_variant(
        &self,
        file: &Path,
        extra_args: &[&str],
        output: &Path,
        harness: &HarnessMetadata,
    ) -> Result<()> {
        let mut args = self.cbmc_flags(file, harness)?;
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
