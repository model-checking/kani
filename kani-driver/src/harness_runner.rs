// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{Result, bail};
use kani_metadata::{ArtifactType, HarnessMetadata};
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::args::OutputFormat;
use crate::call_cbmc::{VerificationResult, VerificationStatus};
use crate::project::Project;
use crate::session::KaniSession;

use std::env::current_dir;
use std::path::PathBuf;

/// A HarnessRunner is responsible for checking all proof harnesses. The data in this structure represents
/// "background information" that the controlling driver (e.g. cargo-kani or kani) computed.
///
/// This struct is basically just a nicer way of passing many arguments to [`Self::check_all_harnesses`]
pub(crate) struct HarnessRunner<'sess, 'pr> {
    /// The underlying kani session
    pub sess: &'sess KaniSession,
    /// The project under verification.
    pub project: &'pr Project,
}

/// The result of checking a single harness. This both hangs on to the harness metadata
/// (as a means to identify which harness), and provides that harness's verification result.
pub(crate) struct HarnessResult<'pr> {
    pub harness: &'pr HarnessMetadata,
    pub result: VerificationResult,
}

impl<'pr> HarnessRunner<'_, 'pr> {
    /// Given a [`HarnessRunner`] (to abstract over how these harnesses were generated), this runs
    /// the proof-checking process for each harness in `harnesses`.
    pub(crate) fn check_all_harnesses(
        &self,
        harnesses: &'pr [&HarnessMetadata],
    ) -> Result<Vec<HarnessResult<'pr>>> {
        self.check_stubbing(harnesses)?;

        let sorted_harnesses = crate::metadata::sort_harnesses_by_loc(harnesses);
        let pool = {
            let mut builder = rayon::ThreadPoolBuilder::new();
            if let Some(x) = self.sess.args.jobs() {
                builder = builder.num_threads(x);
            }
            builder.build()?
        };

        let results = pool.install(|| -> Result<Vec<HarnessResult<'pr>>> {
            sorted_harnesses
                .par_iter()
                .map(|harness| -> Result<HarnessResult<'pr>> {
                    let goto_file =
                        self.project.get_harness_artifact(&harness, ArtifactType::Goto).unwrap();

                    self.sess.instrument_model(goto_file, goto_file, &self.project, &harness)?;

                    if self.sess.args.synthesize_loop_contracts {
                        self.sess.synthesize_loop_contracts(goto_file, &goto_file, &harness)?;
                    }

                    let result = self.sess.check_harness(goto_file, harness)?;
                    Ok(HarnessResult { harness, result })
                })
                .collect::<Result<Vec<_>>>()
        })?;

        Ok(results)
    }

    /// Return an error if the user is trying to verify a harness with stubs without enabling the
    /// experimental feature.
    fn check_stubbing(&self, harnesses: &[&HarnessMetadata]) -> Result<()> {
        if !self.sess.args.is_stubbing_enabled() {
            let with_stubs: Vec<_> = harnesses
                .iter()
                .filter_map(|harness| {
                    (!harness.attributes.stubs.is_empty()).then_some(harness.pretty_name.as_str())
                })
                .collect();
            match with_stubs.as_slice() {
                [] => { /* do nothing */ }
                [harness] => bail!(
                    "Use of unstable feature 'stubbing' in harness `{}`.\n\
                    To enable stubbing, pass option `-Z stubbing`",
                    harness
                ),
                harnesses => bail!(
                    "Use of unstable feature 'stubbing' in harnesses `{}`.\n\
                    To enable stubbing, pass option `-Z stubbing`",
                    harnesses.join("`, `")
                ),
            }
        }
        Ok(())
    }
}

impl KaniSession {
    fn process_output(&self, result: &VerificationResult, harness: &HarnessMetadata) {
        if self.should_print_output() {
            if self.args.output_into_files {
                self.write_output_to_file(result, harness);
            }

            let output = result.render(&self.args.output_format, harness.attributes.should_panic);
            println!("{}", output);
        }
    }

    fn should_print_output(&self) -> bool {
        !self.args.common_args.quiet && self.args.output_format != OutputFormat::Old
    }

    fn write_output_to_file(&self, result: &VerificationResult, harness: &HarnessMetadata) {
        let target_dir = self.result_output_dir().unwrap();
        let file_name = target_dir.join(harness.pretty_name.clone());
        let path = Path::new(&file_name);
        let prefix = path.parent().unwrap();

        std::fs::create_dir_all(prefix).unwrap();
        let mut file = File::create(&file_name).unwrap();
        let file_output = result.render(&OutputFormat::Regular, harness.attributes.should_panic);

        if let Err(e) = writeln!(file, "{}", file_output) {
            eprintln!(
                "Failed to write to file {}: {}",
                file_name.into_os_string().into_string().unwrap(),
                e
            );
        }
    }

    fn result_output_dir(&self) -> Result<PathBuf> {
        let target_dir = self.args.target_dir.clone().map_or_else(current_dir, Ok)?;
        Ok(target_dir.join("result_output_dir")) //Hardcode output to result_output_dir, may want to make it adjustable?
    }

    /// Run the verification process for a single harness
    pub(crate) fn check_harness(
        &self,
        binary: &Path,
        harness: &HarnessMetadata,
    ) -> Result<VerificationResult> {
        if !self.args.common_args.quiet {
            println!("Checking harness {}...", harness.pretty_name);
        }

        let mut result = self.with_timer(|| self.run_cbmc(binary, harness), "run_cbmc")?;

        self.process_output(&result, harness);
        self.gen_and_add_concrete_playback(harness, &mut result)?;
        Ok(result)
    }

    /// Concludes a session by printing a summary report and exiting the process with an
    /// error code (if applicable).
    ///
    /// Note: Takes `self` "by ownership". This function wants to be able to drop before
    /// exiting with an error code, if needed.
    pub(crate) fn print_final_summary(self, results: &[HarnessResult<'_>]) -> Result<()> {
        let (successes, failures): (Vec<_>, Vec<_>) =
            results.iter().partition(|r| r.result.status == VerificationStatus::Success);

        let succeeding = successes.len();
        let failing = failures.len();
        let total = succeeding + failing;

        if self.args.concrete_playback.is_some()
            && !self.args.common_args.quiet
            && results.iter().all(|r| !r.result.generated_concrete_test)
        {
            println!(
                "INFO: The concrete playback feature never generated unit tests because there were no failing harnesses."
            )
        }

        // We currently omit a summary if there was just 1 harness
        if !self.args.common_args.quiet {
            if failing > 0 {
                println!("Summary:");
            }
            for failure in failures.iter() {
                println!("Verification failed for - {}", failure.harness.pretty_name);
            }

            if total > 0 {
                println!(
                    "Complete - {succeeding} successfully verified harnesses, {failing} failures, {total} total."
                );
            } else {
                match self.args.harnesses.as_slice() {
                    [] =>
                    // TODO: This could use a better message, possibly with links to Kani documentation.
                    // New users may encounter this and could use a pointer to how to write proof harnesses.
                    {
                        println!(
                            "No proof harnesses (functions with #[kani::proof]) were found to verify."
                        )
                    }
                    [harness] => {
                        bail!("no harnesses matched the harness filter: `{harness}`")
                    }
                    harnesses => bail!(
                        "no harnesses matched the harness filters: `{}`",
                        harnesses.join("`, `")
                    ),
                };
            }
        }

        if self.args.coverage {
            self.show_coverage_summary()?;
        }

        if failing > 0 {
            // Failure exit code without additional error message
            drop(self);
            std::process::exit(1);
        }

        Ok(())
    }

    /// Show a coverage summary.
    ///
    /// This is just a placeholder for now.
    fn show_coverage_summary(&self) -> Result<()> {
        Ok(())
    }
}
