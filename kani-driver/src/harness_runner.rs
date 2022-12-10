// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use kani_metadata::HarnessMetadata;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

use crate::args::OutputFormat;
use crate::call_cbmc::{VerificationResult, VerificationStatus};
use crate::session::KaniSession;
use crate::util::specialized_harness_name;

/// A HarnessRunner is responsible for checking all proof harnesses. The data in this structure represents
/// "background information" that the controlling driver (e.g. cargo-kani or kani) computed.
///
/// This struct is basically just a nicer way of passing many arguments to [`Self::check_all_harnesses`]
pub(crate) struct HarnessRunner<'sess> {
    /// The underlying kani session
    pub sess: &'sess KaniSession,
    /// The build CBMC goto binary for the "whole program" (will be specialized to each proof harness)
    pub linked_obj: &'sess Path,
    /// The directory we should output cbmc-viewer reports to
    pub report_base: &'sess Path,
    /// An unfortunate behavior difference between `kani` and `cargo kani`: `cargo kani` never deletes the specialized goto binaries, while `kani` does unless `--keep-temps` is provided
    pub retain_specialized_harnesses: bool,

    /// The collection of symtabs that went into the goto binary
    /// (TODO: this is only for --gen-c, which possibly should not be done here (i.e. not from within harness running)?
    ///        <https://github.com/model-checking/kani/pull/1684>)
    pub symtabs: &'sess [PathBuf],
}

/// The result of checking a single harness. This both hangs on to the harness metadata
/// (as a means to identify which harness), and provides that harness's verification result.
pub(crate) struct HarnessResult<'sess> {
    pub harness: &'sess HarnessMetadata,
    pub result: VerificationResult,
}

impl<'sess> HarnessRunner<'sess> {
    /// Given a [`HarnessRunner`] (to abstract over how these harnesses were generated), this runs
    /// the proof-checking process for each harness in `harnesses`.
    pub(crate) fn check_all_harnesses<'a>(
        &self,
        harnesses: &'a [HarnessMetadata],
    ) -> Result<Vec<HarnessResult<'a>>> {
        let sorted_harnesses = crate::metadata::sort_harnesses_by_loc(harnesses);

        let pool = {
            let mut builder = rayon::ThreadPoolBuilder::new();
            if let Some(x) = self.sess.args.jobs() {
                builder = builder.num_threads(x);
            }
            builder.build()?
        };

        let results = pool.install(|| -> Result<Vec<HarnessResult<'a>>> {
            sorted_harnesses
                .par_iter()
                .map(|harness| -> Result<HarnessResult<'a>> {
                    let harness_filename = harness.pretty_name.replace("::", "-");
                    let report_dir = self.report_base.join(format!("report-{harness_filename}"));
                    let specialized_obj =
                        specialized_harness_name(self.linked_obj, &harness_filename);
                    if !self.retain_specialized_harnesses {
                        self.sess.record_temporary_files(&[&specialized_obj]);
                    }
                    self.sess.run_goto_instrument(
                        self.linked_obj,
                        &specialized_obj,
                        self.symtabs,
                        &harness.mangled_name,
                    )?;

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
        if !self.args.quiet {
            println!("Checking harness {}...", harness.pretty_name);
        }

        if self.args.visualize {
            self.run_visualize(binary, report_dir, harness)?;
            // Strictly speaking, we're faking success here. This is more "no error"
            Ok(VerificationResult::mock_success())
        } else {
            let result = self.with_timer(|| self.run_cbmc(binary, harness), "run_cmbc")?;

            // When quiet, we don't want to print anything at all.
            // When output is old, we also don't have real results to print.
            if !self.args.quiet && self.args.output_format != OutputFormat::Old {
                println!("{}", result.render(&self.args.output_format));
            }

            Ok(result)
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

        if self.args.concrete_playback.is_some() && !self.args.quiet && failures.is_empty() {
            println!(
                "INFO: The concrete playback feature never generated unit tests because there were no failing harnesses."
            )
        }

        // We currently omit a summary if there was just 1 harness
        if !self.args.quiet && !self.args.visualize && total != 1 {
            if failing > 0 {
                println!("Summary:");
            }
            for failure in failures.iter() {
                println!("Verification failed for - {}", failure.harness.pretty_name);
            }

            if total > 0 {
                println!(
                    "Complete - {} successfully verified harnesses, {} failures, {} total.",
                    succeeding, failing, total
                );
            } else {
                // TODO: This could use a better error message, possibly with links to Kani documentation.
                // New users may encounter this and could use a pointer to how to write proof harnesses.
                println!(
                    "No proof harnesses (functions with #[kani::proof]) were found to verify."
                );
            }
        }

        #[cfg(feature = "unsound_experiments")]
        self.args.unsound_experiments.print_warnings();

        if failing > 0 {
            // Failure exit code without additional error message
            drop(self);
            std::process::exit(1);
        }

        Ok(())
    }
}
