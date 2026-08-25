// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{Error, Result};
use kani_metadata::{ArtifactType, HarnessKind, HarnessMetadata};
use rayon::prelude::*;
use std::fs::File;
use std::io::{IsTerminal, Write};
use std::path::Path;

use crate::args::{NumThreads, OutputFormat};
use crate::call_cbmc::{VerificationResult, VerificationStatus};
use crate::frontend::{JsonHandler, schema_utils::add_runner_results_to_json};
use crate::progress_indicator::ProgressIndicator;
use crate::project::Project;
use crate::session::{BUG_REPORT_URL, KaniSession};

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

#[derive(Debug)]
struct FailFastHarnessInfo {
    pub index_to_failing_harness: usize,
    pub result: VerificationResult,
}

impl std::error::Error for FailFastHarnessInfo {}

impl std::fmt::Display for FailFastHarnessInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "harness failed")
    }
}

impl<'pr> HarnessRunner<'_, 'pr> {
    /// Given a [`HarnessRunner`] (to abstract over how these harnesses were generated), this runs
    /// the proof-checking process for each harness in `harnesses`.
    pub(crate) fn check_all_harnesses(
        &self,
        harnesses: &'pr [&HarnessMetadata],
        mut json_handler: Option<&mut JsonHandler>,
    ) -> Result<Vec<HarnessResult<'pr>>> {
        let sorted_harnesses = crate::metadata::sort_harnesses_by_loc(harnesses);

        // Determine if we should show progress indicator.
        //
        // Test stderr, not stdout: `ProgressBar::new` draws to stderr
        // (`ProgressDrawTarget::stderr()`), so gating on stdout meant
        // `kani --log-file log.txt > out.txt` from a terminal lost the progress bar
        // even though stderr was still interactive — the very case where redirecting
        // stdout makes the bar most useful. indicatif hides a non-terminal target
        // itself, so this is about not suppressing a bar that would render fine.
        let show_progress = self.sess.args.log_file.is_some()
            && !self.sess.args.common_args.quiet
            && std::io::stderr().is_terminal();

        // Create progress indicator
        let progress_indicator = ProgressIndicator::new(sorted_harnesses.len(), show_progress);

        let pool = {
            let mut builder = rayon::ThreadPoolBuilder::new();
            match self.sess.args.jobs() {
                NumThreads::UserSpecified(num_threads) => {
                    builder = builder.num_threads(num_threads);
                }
                NumThreads::NoMultithreading => {
                    builder = builder.num_threads(1);
                }
                NumThreads::ThreadPoolDefault => { /* rayon will automatically set num_threads to the default if not specified here */
                }
            }
            builder.build()?
        };

        let results = pool.install(|| -> Result<Vec<HarnessResult<'pr>>> {
            sorted_harnesses
                .par_iter()
                .enumerate()
                .map(|(idx, harness)| -> Result<HarnessResult<'pr>> {
                    let goto_file =
                        self.project.get_harness_artifact(harness, ArtifactType::Goto).unwrap();

                    self.sess.instrument_model(goto_file, goto_file, self.project, harness)?;

                    if self.sess.args.synthesize_loop_contracts {
                        self.sess.synthesize_loop_contracts(goto_file, goto_file, harness)?;
                    }

                    let result = self.sess.check_harness(goto_file, harness)?;

                    // Update progress indicator if active
                    if progress_indicator.is_active() {
                        let succeeded = result.status == VerificationStatus::Success;
                        let timed_out =
                            matches!(&result.results, Err(crate::call_cbmc::ExitStatus::Timeout));
                        progress_indicator.update_with_result(succeeded, timed_out);
                    }

                    if self.sess.args.fail_fast && result.status == VerificationStatus::Failure {
                        Err(Error::new(FailFastHarnessInfo {
                            index_to_failing_harness: idx,
                            result,
                        }))
                    } else {
                        Ok(HarnessResult { harness, result })
                    }
                })
                .collect::<Result<Vec<_>>>()
        });

        // Finish progress indicator
        progress_indicator.finish();

        match results {
            Ok(results) => {
                if let Some(handler) = json_handler.as_deref_mut() {
                    add_runner_results_to_json(handler, &results, harnesses.len(), "completed");
                }
                Ok(results)
            }
            Err(err) => {
                if err.is::<FailFastHarnessInfo>() {
                    let failed = err.downcast::<FailFastHarnessInfo>().unwrap();
                    let result = vec![HarnessResult {
                        harness: sorted_harnesses[failed.index_to_failing_harness],
                        result: failed.result,
                    }];

                    if let Some(handler) = json_handler {
                        add_runner_results_to_json(
                            handler,
                            &result,
                            harnesses.len(),
                            "completed_with_fail_fast",
                        );
                    }

                    Ok(result)
                } else {
                    Err(err)
                }
            }
        }
    }
}

impl KaniSession {
    fn process_output(
        &self,
        result: &VerificationResult,
        harness: &HarnessMetadata,
        thread_index: usize,
    ) {
        if self.should_print_output() {
            if self.args.output_into_files {
                self.write_output_to_file(result, harness, thread_index);
            }

            let output = result.render(&self.args.output_format(), harness.attributes.should_panic);

            if rayon::current_num_threads() > 1 {
                self.emit_line(&format!("Thread {thread_index}: {output}"));
            } else {
                self.emit_line(&output);
            }
        }
    }

    /// Emit one line of harness output: to `--log-file` when one is configured, so the
    /// terminal is left to the progress indicator, and to stdout otherwise.
    ///
    /// `line` is emitted as given. Callers own any `Thread N:` prefix, so that the
    /// log file and stdout carry identical text.
    fn emit_line(&self, line: &str) {
        if let Some(ref log_file_path) = self.args.log_file {
            if let Err(e) = crate::log_file::append_line(log_file_path, line) {
                eprintln!("Failed to write to log file {}: {}", log_file_path.display(), e);
            }
        } else {
            println!("{line}");
        }
    }

    fn should_print_output(&self) -> bool {
        !self.args.common_args.quiet && self.args.output_format() != OutputFormat::Old
    }

    fn write_output_to_file(
        &self,
        result: &VerificationResult,
        harness: &HarnessMetadata,
        thread_index: usize,
    ) {
        let target_dir = self.result_output_dir().unwrap();
        let file_name = target_dir.join(harness.pretty_name.clone());
        let path = Path::new(&file_name);
        let prefix = path.parent().unwrap();

        std::fs::create_dir_all(prefix).unwrap();
        let mut file = File::create(&file_name).unwrap();
        let mut file_output =
            result.render(&OutputFormat::Regular, harness.attributes.should_panic);
        if rayon::current_num_threads() > 1 {
            file_output = format!("Thread {thread_index}:\n{file_output}");
        }

        if let Err(e) = writeln!(file, "{file_output}") {
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
        let thread_index = rayon::current_thread_index().unwrap_or_default();
        if !self.args.common_args.quiet {
            // If the harness is automatically generated, pretty_name refers to the function under verification.
            let mut msg = if harness.is_automatically_generated {
                // A bounded harness only explores some arguments up to a bound, so it does not
                // check *all* possible inputs; qualify the message accordingly so a bounded run
                // does not masquerade as exhaustive (c.f. the "(bounded)" marker in the summary).
                let inputs = if harness.is_bounded {
                    "all possible inputs (bounded for some arguments)"
                } else {
                    "all possible inputs"
                };
                if matches!(harness.attributes.kind, HarnessKind::Proof) {
                    format!(
                        "Autoharness: Checking function {} against {inputs}...",
                        harness.pretty_name
                    )
                } else {
                    format!(
                        "Autoharness: Checking function {}'s contract against {inputs}...",
                        harness.pretty_name
                    )
                }
            } else {
                format!("Checking harness {}...", harness.pretty_name)
            };

            if rayon::current_num_threads() > 1 {
                msg = format!("Thread {thread_index}: {msg}");
            }

            self.emit_line(&msg);

            // Print stubs applied to this harness so users know which
            // assumptions are in effect.
            let multi = rayon::current_num_threads() > 1;
            let print_line = |line: String| {
                if multi {
                    self.emit_line(&format!("Thread {thread_index}: {line}"));
                } else {
                    self.emit_line(&line);
                }
            };
            for stub in &harness.attributes.stubs {
                print_line(format!("  - Stub: {} -> {}", stub.original, stub.replacement));
            }
            for verified in &harness.attributes.verified_stubs {
                print_line(format!("  - Verified stub: {verified}"));
            }
        }

        let mut result = self.with_timer(|| self.run_cbmc(binary, harness), "run_cbmc")?;

        self.process_output(&result, harness, thread_index);
        self.gen_and_add_concrete_playback(harness, &mut result)?;
        Ok(result)
    }

    /// Concludes a session by printing a summary report and exiting the process with an
    /// error code (if applicable).
    ///
    /// Note: Takes `self` "by ownership". This function wants to be able to drop before
    /// exiting with an error code, if needed.
    pub(crate) fn print_final_summary(self, results: &[HarnessResult<'_>]) -> Result<()> {
        if self.args.common_args.quiet {
            return Ok(());
        }

        let (automatic, manual): (Vec<_>, Vec<_>) =
            results.iter().partition(|r| r.harness.is_automatically_generated);

        let (successes, failures): (Vec<_>, Vec<_>) =
            manual.into_iter().partition(|r| r.result.status == VerificationStatus::Success);

        let succeeding = successes.len();
        let failing = failures.len();
        let total = succeeding + failing;

        if self.args.concrete_playback.is_some() {
            if failures.is_empty() {
                println!(
                    "INFO: The concrete playback feature never generated unit tests because there were no failing harnesses."
                )
            } else if failures.iter().all(|r| !r.result.generated_concrete_test) {
                eprintln!(
                    "The concrete playback feature did not generate unit tests, but there were failing harnesses. Please file a bug report at {BUG_REPORT_URL}"
                )
            }
        }

        println!("Manual Harness Summary:");

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
                // `determine_targets` fails a zero-match filter before codegen, so this arm
                // only guards paths that skip harness filtering.
                _ => return Err(crate::metadata::no_harness_match_error(&self.args.harnesses)),
            };
        }

        if self.args.coverage {
            self.show_coverage_summary()?;
        }

        let autoharness_failing = if self.autoharness_compiler_flags.is_some() {
            self.print_autoharness_summary(automatic)?
        } else {
            0
        };

        if failing + autoharness_failing > 0 {
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
