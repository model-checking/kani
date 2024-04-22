// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fs;
use std::fs::File;
use std::io::Write;

use crate::harness_runner::HarnessResult;
use crate::KaniSession;
use anyhow::Result;

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
