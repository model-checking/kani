// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::str::FromStr;

use crate::args::Timeout;
use crate::call_cbmc::VerificationStatus;
use crate::call_single_file::to_rustc_arg;
use crate::harness_runner::HarnessResult;
use crate::session::KaniSession;
use anyhow::Result;

const AUTOHARNESS_TIMEOUT: &str = "30s";
const LOOP_UNWIND_DEFAULT: u32 = 20;

impl KaniSession {
    /// Enable autoharness mode.
    pub fn enable_autoharness(&mut self) {
        self.auto_harness = true;
    }

    /// Add the compiler arguments specific to the `autoharness` subcommand.
    pub fn add_auto_harness_args(&mut self, included: Vec<String>, excluded: Vec<String>) {
        for func in included {
            self.pkg_args
                .push(to_rustc_arg(vec![format!("--autoharness-include-function {}", func)]));
        }
        for func in excluded {
            self.pkg_args
                .push(to_rustc_arg(vec![format!("--autoharness-exclude-function {}", func)]));
        }
    }

    /// Add global harness timeout and loop unwinding bounds if not provided.
    /// These prevent automatic harnesses from hanging.
    pub fn add_default_bounds(&mut self) {
        if self.args.harness_timeout.is_none() {
            let timeout = Timeout::from_str(AUTOHARNESS_TIMEOUT).unwrap();
            self.args.harness_timeout = Some(timeout);
        }
        if self.args.default_unwind.is_none() {
            self.args.default_unwind = Some(LOOP_UNWIND_DEFAULT);
        }
    }

    /// Prints the results from running the `autoharness` subcommand.
    pub fn print_autoharness_summary(&self, automatic: Vec<&HarnessResult<'_>>) -> Result<()> {
        let (successes, failures): (Vec<_>, Vec<_>) =
            automatic.into_iter().partition(|r| r.result.status == VerificationStatus::Success);

        let succeeding = successes.len();
        let failing = failures.len();
        let total = succeeding + failing;

        // TODO: it would be nice if we had access to which functions the user included/excluded here
        // so that we could print a comparison for them of any of the included functions that we skipped.
        println!("Autoharness Summary:");
        println!(
            "Note that Kani will only generate an automatic harness for a function if it determines that each of its arguments implement the Arbitrary trait."
        );
        println!(
            "Examine the summary closely to determine which functions were automatically verified."
        );
        if failing > 0 {
            println!(
                "Also note that Kani sets default --harness-timeout of {AUTOHARNESS_TIMEOUT} and --default-unwind of {LOOP_UNWIND_DEFAULT}."
            );
            println!(
                "If verification failed because of timing out or too low of an unwinding bound, try passing larger values for these arguments (or, if possible, writing a loop contract)."
            );
        }

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
            println!(
                "No functions were eligible for automatic verification. Functions can only be automatically verified if each of their arguments implement kani::Arbitrary."
            );
            println!(
                "If you specified --include-function or --exclude-function, make sure that your filters were not overly restrictive."
            );
        }

        // Manual harness summary may come afterward, so separate them with a new line.
        println!();
        Ok(())
    }
}
