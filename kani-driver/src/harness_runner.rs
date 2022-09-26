// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use kani_metadata::HarnessMetadata;
use std::path::{Path, PathBuf};

use crate::call_cbmc::VerificationStatus;
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
    pub result: VerificationStatus,
}

/// The results of checking all harnesses. In addition to keeping the result information
/// in more detail per-harness, the harnesses are partitioned into `successes` and `failures`.
#[derive(Default)]
pub(crate) struct HarnessResults<'sess> {
    pub successes: Vec<HarnessResult<'sess>>,
    pub failures: Vec<HarnessResult<'sess>>,
}

impl<'sess> HarnessRunner<'sess> {
    /// Given a [`HarnessRunner`] (to abstract over how these harnesses were generated), this runs
    /// the proof-checking process for each harness in `harnesses`.
    pub(crate) fn check_all_harnesses<'a>(
        &self,
        harnesses: &'a [HarnessMetadata],
    ) -> Result<HarnessResults<'a>> {
        let sorted_harnesses = crate::metadata::sort_harnesses_by_loc(harnesses);

        let mut results = HarnessResults::default();

        for harness in &sorted_harnesses {
            let harness_filename = harness.pretty_name.replace("::", "-");
            let report_dir = self.report_base.join(format!("report-{}", harness_filename));
            let specialized_obj = specialized_harness_name(self.linked_obj, &harness_filename);
            if !self.retain_specialized_harnesses {
                let mut temps = self.sess.temporaries.borrow_mut();
                temps.push(specialized_obj.to_owned());
            }
            self.sess.run_goto_instrument(
                self.linked_obj,
                &specialized_obj,
                self.symtabs,
                &harness.mangled_name,
            )?;

            let result = self.sess.check_harness(&specialized_obj, &report_dir, harness)?;
            let wrapped_result = HarnessResult { harness, result };
            if wrapped_result.result == VerificationStatus::Failure {
                results.failures.push(wrapped_result);
            } else {
                results.successes.push(wrapped_result);
            }
        }

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
    ) -> Result<VerificationStatus> {
        if !self.args.quiet {
            println!("Checking harness {}...", harness.pretty_name);
        }

        if self.args.visualize {
            self.run_visualize(binary, report_dir, harness)?;
            // Strictly speaking, we're faking success here. This is more "no error"
            Ok(VerificationStatus::Success)
        } else {
            self.run_cbmc(binary, harness)
        }
    }

    /// Concludes a session by printing a summary report and exiting the process with an
    /// error code (if applicable).
    ///
    /// Note: Takes `self` "by ownership". This function wants to be able to drop before
    /// exiting with an error code, if needed.
    pub(crate) fn print_final_summary(self, results: &HarnessResults) -> Result<()> {
        let succeeding = results.successes.len();
        let failing = results.failures.len();
        let total = succeeding + failing;
        if !self.args.quiet && !self.args.visualize && total > 1 {
            if failing > 0 {
                println!("Summary:");
            }
            for failure in results.failures.iter() {
                println!("Verification failed for - {}", failure.harness.pretty_name);
            }

            println!(
                "Complete - {} successfully verified harnesses, {} failures, {} total.",
                succeeding, failing, total
            );
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
