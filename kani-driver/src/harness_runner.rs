// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use kani_metadata::{ArtifactType, HarnessMetadata};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::path::Path;

use crate::args::OutputFormat;
use crate::call_cbmc::{VerificationResult, VerificationStatus};
use crate::project::Project;
use crate::session::KaniSession;
use crate::util::{error, specialized_harness_name, warning};

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

impl<'sess, 'pr> HarnessRunner<'sess, 'pr> {
    /// Given a [`HarnessRunner`] (to abstract over how these harnesses were generated), this runs
    /// the proof-checking process for each harness in `harnesses`.
    pub(crate) fn check_all_harnesses(
        &self,
        harnesses: &'pr [&HarnessMetadata],
    ) -> Result<Vec<HarnessResult<'pr>>> {
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
                    let harness_filename = harness.pretty_name.replace("::", "-");
                    let report_dir = self.project.outdir.join(format!("report-{harness_filename}"));
                    let goto_file =
                        self.project.get_harness_artifact(&harness, ArtifactType::Goto).unwrap();
                    let specialized_obj = specialized_harness_name(goto_file, &harness_filename);
                    self.sess.record_temporary_file(&specialized_obj);
                    self.sess.instrument_model(
                        goto_file,
                        &specialized_obj,
                        &self.project,
                        &harness,
                    )?;

                    if self.sess.args.synthesize_loop_contracts {
                        self.sess.synthesize_loop_contracts(
                            &specialized_obj,
                            &specialized_obj,
                            &harness,
                        )?;
                    }

                    let result = self.sess.check_harness(&specialized_obj, &report_dir, harness)?;
                    Ok(HarnessResult { harness, result })
                })
                .collect::<Result<Vec<_>>>()
        })?;

        Ok(results)
    }
}

impl KaniSession {
    /// Run the verification process for a single harness
    pub(crate) fn check_harness(
        &self,
        binary: &Path,
        report_dir: &Path,
        harness: &HarnessMetadata,
    ) -> Result<VerificationResult> {
        if !self.args.common_args.quiet {
            println!("Checking harness {}...", harness.pretty_name);
        }

        if self.args.visualize {
            self.run_visualize(binary, report_dir, harness)?;
            // Strictly speaking, we're faking success here. This is more "no error"
            Ok(VerificationResult::mock_success())
        } else {
            let result = self.with_timer(|| self.run_cbmc(binary, harness), "run_cbmc")?;

            // When quiet, we don't want to print anything at all.
            // When output is old, we also don't have real results to print.
            if !self.args.common_args.quiet && self.args.output_format != OutputFormat::Old {
                println!(
                    "{}",
                    result.render(&self.args.output_format, harness.attributes.should_panic)
                );
            }

            Ok(result)
        }
    }

    /// Prints a warning at the end of the verification if harness contained a stub but stubs were
    /// not enabled.
    fn stubbing_statuses(&self, results: &[HarnessResult]) {
        if !self.args.enable_stubbing {
            let ignored_stubs: Vec<_> = results
                .iter()
                .filter_map(|result| {
                    (!result.harness.attributes.stubs.is_empty())
                        .then_some(result.harness.pretty_name.as_str())
                })
                .collect();
            match ignored_stubs.len().cmp(&1) {
                Ordering::Equal => warning(&format!(
                    "harness `{}` contained stubs which were ignored.\n\
                    To enable stubbing, pass options `--enable-unstable --enable-stubbing`",
                    ignored_stubs[0]
                )),
                Ordering::Greater => warning(&format!(
                    "harnesses `{}` contained stubs which were ignored.\n\
                    To enable stubbing, pass options `--enable-unstable --enable-stubbing`",
                    ignored_stubs.join("`, `")
                )),
                Ordering::Less => {}
            }
        }
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
        if !self.args.common_args.quiet && !self.args.visualize && total != 1 {
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
                match (self.args.harnesses.as_slice(), &self.args.function) {
                    ([], None) =>
                    // TODO: This could use a better message, possibly with links to Kani documentation.
                    // New users may encounter this and could use a pointer to how to write proof harnesses.
                    {
                        println!(
                            "No proof harnesses (functions with #[kani::proof]) were found to verify."
                        )
                    }
                    ([harness], None) => {
                        bail!("no harnesses matched the harness filter: `{harness}`")
                    }
                    (harnesses, None) => bail!(
                        "no harnesses matched the harness filters: `{}`",
                        harnesses.join("`, `")
                    ),
                    ([], Some(func)) => error(&format!("No function named {func} was found")),
                    _ => unreachable!(
                        "invalid configuration. Cannot specify harness and function at the same time"
                    ),
                };
            }
        }

        self.stubbing_statuses(results);

        if failing > 0 {
            // Failure exit code without additional error message
            drop(self);
            std::process::exit(1);
        }

        Ok(())
    }
}
