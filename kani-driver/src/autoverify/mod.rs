// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::call_cbmc::VerificationStatus;
use crate::call_single_file::to_rustc_arg;
use crate::harness_runner::HarnessResult;
use crate::session::KaniSession;
use anyhow::Result;

impl KaniSession {
    /// Enable autoverify mode.
    pub fn enable_autoverify(&mut self) {
        self.auto_verify = true;
    }

    /// Add the compiler arguments specific to the `autoverify` subcommand.
    pub fn add_auto_verify_args(&mut self, included: Vec<String>, excluded: Vec<String>) {
        for func in included {
            self.pkg_args
                .push(to_rustc_arg(vec![format!("--autoverify-include-function {}", func)]));
        }
        for func in excluded {
            self.pkg_args
                .push(to_rustc_arg(vec![format!("--autoverify-exclude-function {}", func)]));
        }
    }

    /// Prints the results from running the `autoverify` subcommand.
    pub fn print_autoverify_summary(&self, automatic: Vec<&HarnessResult<'_>>) -> Result<()> {
        let (successes, failures): (Vec<_>, Vec<_>) =
            automatic.into_iter().partition(|r| r.result.status == VerificationStatus::Success);

        let succeeding = successes.len();
        let failing = failures.len();
        let total = succeeding + failing;

        // TODO: it would be nice if we had access to which functions the user included/excluded here
        // so that we could print a comparison for them of any of the included functions that we skipped.
        println!("Autoverify Summary:");
        println!(
            "Note that Kani will only autoverify a function if it determines that each of its arguments implement the Arbitrary trait."
        );
        println!(
            "Examine the summary closely to determine which functions were automatically verified."
        );

        // Since autoverification skips over some functions, print the successes to make it easier to see what we verified in one place.
        for success in successes {
            println!("Verification succeeded for - {}", success.harness.pretty_name);
        }

        for failure in failures {
            println!("Verification failed for - {}", failure.harness.pretty_name);
        }

        if total > 0 {
            println!(
                "Complete - {succeeding} successfully verified functions, {failing} failures, {total} total."
            );
        } else {
            println!("
                No functions were eligible for automatic verification. Functions can only be automatically verified if each of their arguments implement kani::Arbitrary.");
            println!(
                "If you specified --include-function or --exclude-function, make sure that your filters were not overly restrictive."
            );
        }

        // Manual harness summary may come afterward, so separate them with a new line.
        println!();
        Ok(())
    }
}
