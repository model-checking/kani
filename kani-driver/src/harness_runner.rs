// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use kani_metadata::{ArtifactType, HarnessKind, HarnessMetadata};
use rayon::prelude::*;
use std::fs::File;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

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

/// The outcome of one unit of work run by [`run_until_abort`].
struct Completed<T> {
    payload: T,
    /// Whether this outcome should stop units that have not started yet.
    aborts: bool,
}

/// Restore input order over payloads tagged with the index they came from.
///
/// Units finish in whatever order the thread pool gets to them, so without this the results
/// would be reported in completion order and would differ run to run.
fn in_input_order<T>(mut tagged: Vec<(usize, T)>) -> Vec<T> {
    tagged.sort_by_key(|(idx, _)| *idx);
    tagged.into_iter().map(|(_, payload)| payload).collect()
}

/// Run `run_one` over `items` in parallel, keeping the payloads of units that completed even when
/// one of them aborts the run, and returning them in `items` order together with whether an abort
/// happened.
///
/// The abort is latched rather than raised through the `Err` channel, which stays reserved for
/// genuine errors. `try_for_each` surfaces only one `Err`, so signalling an abort that way would
/// let it displace a real failure raised by a sibling unit still in flight.
fn run_until_abort<I: Sync, T: Send>(
    items: &[I],
    run_one: impl Fn(&I) -> Result<Completed<T>> + Sync,
) -> Result<(Vec<T>, bool)> {
    let completed: Mutex<Vec<(usize, T)>> = Mutex::new(Vec::new());
    let aborted = AtomicBool::new(false);

    items.par_iter().enumerate().try_for_each(|(idx, item)| -> Result<()> {
        // Best effort: a unit that has not started returns without running once it observes a
        // latched abort. The load is `Relaxed`, so a unit racing the store may still run; that
        // costs one extra unit of work and is not incorrect. Units already in flight finish and
        // record their payloads.
        if aborted.load(Ordering::Relaxed) {
            return Ok(());
        }

        let Completed { payload, aborts } = run_one(item)?;
        completed.lock().unwrap().push((idx, payload));

        if aborts {
            aborted.store(true, Ordering::Relaxed);
        }
        Ok(())
    })?;

    let payloads = in_input_order(completed.into_inner().unwrap());

    // Read after the parallel region has joined, which orders every store before this load.
    Ok((payloads, aborted.load(Ordering::Relaxed)))
}

impl<'pr> HarnessRunner<'_, 'pr> {
    /// Given a [`HarnessRunner`] (to abstract over how these harnesses were generated), this runs
    /// the proof-checking process for each harness in `harnesses`.
    pub(crate) fn check_all_harnesses(
        &self,
        harnesses: &'pr [&HarnessMetadata],
        json_handler: Option<&mut JsonHandler>,
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

        let run_result = pool.install(|| {
            run_until_abort(&sorted_harnesses, |harness| {
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

                let aborts =
                    self.sess.args.fail_fast && result.status == VerificationStatus::Failure;

                Ok(Completed { payload: HarnessResult { harness, result }, aborts })
            })
        });

        // Finish progress indicator
        progress_indicator.finish();

        // The `Err` channel carries genuine errors only, so any error propagates.
        let (results, fail_fast) = run_result?;

        if let Some(handler) = json_handler {
            let status_label = if fail_fast { "completed_with_fail_fast" } else { "completed" };
            add_runner_results_to_json(handler, &results, harnesses.len(), status_label);
        }

        Ok(results)
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

            let output = result.render(&self.args.output_format, harness.attributes.should_panic);

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
        !self.args.common_args.quiet && self.args.output_format != OutputFormat::Old
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

#[cfg(test)]
mod tests {
    use super::{Completed, in_input_order, run_until_abort};
    use anyhow::{Result, anyhow};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    /// What one unit of work should do when it runs.
    #[derive(Clone, Copy)]
    enum Unit {
        Pass,
        /// Completes, and latches the abort.
        Abort,
        /// Fails for a reason of its own, unrelated to `--fail-fast`.
        Error,
    }

    /// Run `units` on a single thread, so the order they are visited in is the order given.
    fn run_sequential(units: &[Unit]) -> (Result<(Vec<usize>, bool)>, Vec<usize>) {
        run_on(units, 1)
    }

    fn run_on(units: &[Unit], threads: usize) -> (Result<(Vec<usize>, bool)>, Vec<usize>) {
        let ran: Mutex<Vec<usize>> = Mutex::new(Vec::new());
        let indexed: Vec<(usize, Unit)> = units.iter().copied().enumerate().collect();
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();

        let outcome = pool.install(|| {
            run_until_abort(&indexed, |(idx, unit)| {
                ran.lock().unwrap().push(*idx);
                match unit {
                    Unit::Pass => Ok(Completed { payload: *idx, aborts: false }),
                    Unit::Abort => Ok(Completed { payload: *idx, aborts: true }),
                    Unit::Error => Err(anyhow!("unit {idx} failed for its own reasons")),
                }
            })
        });

        let mut ran = ran.into_inner().unwrap();
        ran.sort_unstable();
        (outcome, ran)
    }

    /// The property the whole design rests on: an abort is reported through the returned flag and
    /// never through the `Err` channel.
    ///
    /// This is what makes it impossible for an abort to displace a genuine error. `try_for_each`
    /// surfaces only one `Err`, so as long as an abort never produces one, whatever `Err` comes
    /// back is a real failure. Revert the latch to an `Err(FailFastAbort)` and this test fails.
    #[test]
    fn an_abort_is_not_reported_as_an_error() {
        let (outcome, _) = run_sequential(&[Unit::Abort]);
        let (payloads, aborted) = outcome.expect("an abort must not surface as an error");
        assert!(aborted, "the abort must be reported through the flag");
        assert_eq!(payloads, vec![0], "the aborting unit's own payload is kept");
    }

    /// The reviewer's scenario: one unit trips the abort while another returns a genuine error, both
    /// in flight at once. The genuine error must be what propagates.
    ///
    /// Both units wait for the other to start, so on a pool that runs them in parallel the abort
    /// and the error really do overlap. The wait is bounded, so nothing hangs.
    ///
    /// Rayon is free not to split a two-element iterator, and if it serializes them the aborting
    /// unit would latch the abort before the erroring unit ran, leaving no genuine error to
    /// displace and no way to assert one. So the aborting unit latches the abort only once it has
    /// seen the other unit start. Serialized, it declines to abort and the erroring unit runs
    /// afterwards, so the assertion holds either way rather than failing on a scheduling accident.
    #[test]
    fn a_genuine_error_is_not_displaced_by_a_concurrent_abort() {
        let started = AtomicUsize::new(0);
        let units = [Unit::Abort, Unit::Error];
        let indexed: Vec<(usize, Unit)> = units.iter().copied().enumerate().collect();
        let pool = rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap();

        let outcome = pool.install(|| {
            run_until_abort(&indexed, |(idx, unit)| {
                started.fetch_add(1, Ordering::SeqCst);
                let mut both_in_flight = false;
                for _ in 0..1_000 {
                    if started.load(Ordering::SeqCst) >= 2 {
                        both_in_flight = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                match unit {
                    Unit::Abort => Ok(Completed { payload: *idx, aborts: both_in_flight }),
                    _ => Err(anyhow!("unit {idx} failed for its own reasons")),
                }
            })
        });

        let err = outcome.expect_err("the genuine error must propagate");
        assert_eq!(err.to_string(), "unit 1 failed for its own reasons");
    }

    /// A genuine error propagates when nothing aborts, which is the behaviour that existed before
    /// `--fail-fast` and must not regress.
    #[test]
    fn a_genuine_error_propagates_on_its_own() {
        let (outcome, _) = run_sequential(&[Unit::Pass, Unit::Error]);
        let err = outcome.expect_err("a genuine error must propagate");
        assert_eq!(err.to_string(), "unit 1 failed for its own reasons");
    }

    /// An abort stops units that have not started. The unit after it never runs, which is why the
    /// error it would have raised is absent rather than swallowed.
    #[test]
    fn an_abort_stops_units_that_have_not_started() {
        let (outcome, ran) = run_sequential(&[Unit::Abort, Unit::Error]);
        let (payloads, aborted) = outcome.expect("the skipped unit never ran, so no error exists");
        assert!(aborted);
        assert_eq!(payloads, vec![0]);
        assert_eq!(ran, vec![0], "the unit after the abort must not have run");
    }

    /// Units that completed before the abort keep their payloads, in input order rather than
    /// completion order. This is the original defect the branch fixes.
    #[test]
    fn units_completed_before_an_abort_are_kept_in_order() {
        let (outcome, _) = run_sequential(&[Unit::Pass, Unit::Pass, Unit::Abort, Unit::Pass]);
        let (payloads, aborted) = outcome.expect("an abort is not an error");
        assert!(aborted);
        assert_eq!(payloads, vec![0, 1, 2], "completed payloads kept, in input order");
    }

    /// The ordering step, on an input it can actually fail on.
    ///
    /// The runs above are all on a one-thread pool, where units complete in index order and the
    /// sort is a no-op: delete it and every one of them still passes. Under `--jobs N` completion
    /// order really is shuffled, so drive the sort directly with a shuffled input.
    #[test]
    fn payloads_are_restored_to_input_order() {
        let shuffled = vec![(3, "d"), (0, "a"), (2, "c"), (1, "b")];
        assert_eq!(in_input_order(shuffled), vec!["a", "b", "c", "d"]);
    }

    /// Already-ordered and empty inputs are the boundary cases.
    #[test]
    fn in_input_order_handles_sorted_and_empty_inputs() {
        assert_eq!(in_input_order(vec![(0, "a"), (1, "b")]), vec!["a", "b"]);
        assert!(in_input_order(Vec::<(usize, &str)>::new()).is_empty());
    }

    /// A run where nothing aborts and nothing fails reports no abort and keeps everything.
    #[test]
    fn a_clean_run_reports_no_abort() {
        let (outcome, _) = run_sequential(&[Unit::Pass, Unit::Pass, Unit::Pass]);
        let (payloads, aborted) = outcome.expect("a clean run must succeed");
        assert!(!aborted);
        assert_eq!(payloads, vec![0, 1, 2]);
    }
}
