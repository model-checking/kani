// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fs;
use std::fs::File;
use std::io::Write;

use crate::args::coverage_args::CargoCoverageArgs;
use crate::KaniSession;
use crate::project;
use crate::harness_runner;
use crate::harness_runner::HarnessResult;
use anyhow::Result;
use tracing::debug;


use super::cov_results;

pub fn coverage_cargo(mut session: KaniSession, _args: CargoCoverageArgs) -> Result<()> {
    session.args.coverage = true;
    let project = project::cargo_project(&session, false)?;
    let harnesses = session.determine_targets(&project.get_all_harnesses())?;
    debug!(n = harnesses.len(), ?harnesses, "coverage_cargo");

    // Verification
    let runner = harness_runner::HarnessRunner { sess: &session, project: &project };
    let results = runner.check_all_harnesses(&harnesses)?;

    let _ = session.save_cov_results(&results);

    // More to come later
    Ok(())
}

impl KaniSession {
    pub fn save_cov_results(&self, results: &Vec<HarnessResult>) -> Result<()> {
        let build_target = env!("TARGET");
        let metadata = self.cargo_metadata(build_target)?;
        let target_dir = self
        .args
        .target_dir
        .as_ref()
        .unwrap_or(&metadata.target_directory.clone().into())
        .clone()
        .join("kani");
    
        let outdir = target_dir.join(build_target).join("cov");
    
        if !outdir.exists() {
            fs::create_dir(&outdir)?;
        }

        for harness_res in results {
            let harness_name = harness_res.harness.mangled_name.clone();
            let file_name = outdir.join(harness_name).with_extension("kanicov");
            let mut cov_file = File::create(file_name)?;

            let cov_results = &harness_res.result.coverage_results.clone().unwrap();
            let serialized_data = serde_json::to_string(&cov_results)?;
            cov_file.write_all(serialized_data.as_bytes())?;
        }

        println!("[info] Coverage results saved to {}", &outdir.display());
        Ok(())
    }
}
