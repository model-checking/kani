// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use kani_metadata::HarnessMetadata;
use std::path::{Path, PathBuf};

use crate::call_cbmc::VerificationStatus;
use crate::session::KaniSession;
use crate::util::specialized_harness_name;

pub(crate) struct HarnessRunnerSession<'sess> {
    /// The underlying kani session
    pub sess: &'sess KaniSession,
    /// The build CBMC goto binary for the "whole program" (will be specialized to each proof harness)
    pub linked_obj: &'sess Path,
    /// The directory we should output cbmc-viewer reports to
    pub report_base: &'sess Path,
    /// A behavior difference between `kani` and `cargo kani`: `cargo kani` never deleted the specialized goto binaries
    pub retain_specialized_harnesses: bool,

    /// The collection of symtabs that went into the goto binary
    /// (TODO: this is only for --gen-c, which possibly should not be done here?)
    pub symtabs: &'sess [PathBuf],
}

pub(crate) struct HarnessResult<'sess> {
    pub harness: &'sess HarnessMetadata,
    pub result: VerificationStatus,
}

#[derive(Default)]
pub(crate) struct HarnessResults<'sess> {
    pub successes: Vec<HarnessResult<'sess>>,
    pub failures: Vec<HarnessResult<'sess>>,
}

impl<'sess> HarnessRunnerSession<'sess> {
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
