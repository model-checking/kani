// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{Result, bail};
use kani_metadata::{CbmcSolver, HarnessMetadata};
use regex::Regex;
use rustc_demangle::demangle;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ffi::OsString;
use std::fmt::Write;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use strum_macros::Display;
use tokio::process::Command as TokioCommand;

use crate::args::common::Verbosity;
use crate::args::{OutputFormat, VerificationArgs};
use crate::cbmc_output_parser::{
    CheckStatus, ParserItem, Property, VerificationOutput, extract_results, process_cbmc_output,
};
use crate::cbmc_property_renderer::{format_coverage, format_result, kani_cbmc_output_filter};
use crate::coverage::cov_results::{CoverageCheck, CoverageResults};
use crate::coverage::cov_results::{CoverageRegion, CoverageTerm};
use crate::session::KaniSession;
use crate::util::render_command;

const MAX_WARNING_MESSAGE_CHARS: usize = 4096;
const MAX_WARNINGS_PER_HARNESS: usize = 20;

/// `--export-json`'s `harnesses[].warnings[]` element. `original_chars` is
/// `Some(pre-truncation count)` exactly when `truncated` is `true`.
#[derive(Clone, Debug, Serialize)]
pub struct Warning {
    pub message: String,
    pub truncated: bool,
    pub original_chars: Option<usize>,
}

fn truncate_warning(message: &str) -> Warning {
    let total_chars = message.chars().count();
    if total_chars <= MAX_WARNING_MESSAGE_CHARS {
        return Warning { message: message.to_string(), truncated: false, original_chars: None };
    }
    let prefix: String = message.chars().take(MAX_WARNING_MESSAGE_CHARS).collect();
    Warning { message: prefix, truncated: true, original_chars: Some(total_chars) }
}

/// Bounded warning retention for `--export-json`, built up as messages stream in: at most
/// `MAX_WARNINGS_PER_HARNESS` full `Warning` records, with everything past the cap counted (not
/// cloned) into `truncated` -- so a long-running harness never holds every warning CBMC printed
/// just in case a `--harness-timeout` cancellation needs them.
#[derive(Default)]
struct CapturedWarnings {
    warnings: Vec<Warning>,
    truncated: usize,
}

impl CapturedWarnings {
    fn push(&mut self, message: &str) {
        if self.warnings.len() < MAX_WARNINGS_PER_HARNESS {
            self.warnings.push(truncate_warning(message));
        } else {
            self.truncated += 1;
        }
    }

    fn into_parts(self) -> (Vec<Warning>, usize) {
        (self.warnings, self.truncated)
    }
}

/// Wraps `output_filter` so that, when `export_json` is set, every `WARNING`-typed message it
/// passes through is also retained in the returned handle (capped, see `CapturedWarnings`) --
/// outside the cancellable `process_cbmc_output` future, so a `--harness-timeout` cancellation
/// still has access to whatever was captured before the cut.
fn warning_capturing_filter<F>(
    export_json: bool,
    mut output_filter: F,
) -> (impl FnMut(ParserItem) -> Option<ParserItem>, Arc<Mutex<CapturedWarnings>>)
where
    F: FnMut(ParserItem) -> Option<ParserItem>,
{
    let captured = Arc::new(Mutex::new(CapturedWarnings::default()));
    let capture = captured.clone();
    let filter = move |item| {
        let filtered = output_filter(item);
        if export_json
            && let Some(ParserItem::Message { message_text, message_type }) = &filtered
            && message_type.eq_ignore_ascii_case("warning")
        {
            capture.lock().unwrap().push(message_text);
        }
        filtered
    };
    (filter, captured)
}

/// We will use Cadical by default since it performed better than MiniSAT in our analysis.
/// Note: Kissat was marginally better, but it is an external solver which could be more unstable.
static DEFAULT_SOLVER: CbmcSolver = CbmcSolver::Cadical;

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq)]
pub enum VerificationStatus {
    Success,
    Failure,
}

/// Represents failed properties in three different categories.
/// This simplifies the process to determine and format verification results.
#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailedProperties {
    // No failures
    None,
    // One or more panic-related failures
    PanicsOnly,
    // One or more failures that aren't panic-related
    Other,
    // One or more properties resulted in an ERROR rather than a failing/successful verification
    Error,
}

/// The possible CBMC exit statuses
#[derive(Clone, Copy, Debug)]
pub enum ExitStatus {
    Timeout,
    OutOfMemory,
    /// the integer is the process exit status
    Other(i32),
}

/// Our (kani-driver) notions of CBMC results.
#[derive(Debug)]
pub struct VerificationResult {
    /// Whether verification should be considered to have succeeded, or have failed.
    pub status: VerificationStatus,
    /// The compact representation for failed properties
    pub failed_properties: FailedProperties,
    /// The `Result` properties in detail or the exit_status of CBMC.
    /// Note: CBMC process exit status is only potentially useful if `status` is `Failure`.
    /// Kani will see CBMC report "failure" that's actually success (interpreting "failed"
    /// checks like coverage as expected and desirable.)
    pub results: Result<Vec<Property>, ExitStatus>,
    /// The runtime duration of this CBMC invocation.
    pub runtime: Duration,
    /// Whether concrete playback generated a test
    pub generated_concrete_test: bool,
    /// The number of quantifier expressions CBMC's solver backend could not encode and
    /// dropped (replaced with unconstrained values), c.f. CBMC's "warning: ignoring forall"
    /// messages. A nonzero count makes the analysis unsound -- an `assume` containing such a
    /// quantifier is not enforced (a successful result would be vacuous), and an `assert`
    /// containing one may fail spuriously -- so it forces the verification to fail (see
    /// `VerificationResult::from`).
    pub ignored_quantifiers: usize,
    /// The coverage results
    pub coverage_results: Option<CoverageResults>,
    /// Raw CBMC `WARNING`-type messages, capped; retained only when `--export-json` is set.
    pub warnings: Vec<Warning>,
    pub warnings_truncated: usize,
}

impl KaniSession {
    /// Verify a goto binary that's been prepared with goto-instrument
    pub fn run_cbmc(&self, file: &Path, harness: &HarnessMetadata) -> Result<VerificationResult> {
        let args: Vec<OsString> = self.cbmc_flags(file, harness)?;

        // TODO get cbmc path from self
        let mut cmd = TokioCommand::new("cbmc");
        cmd.args(args);

        let verification_results = if self.args.output_format() == crate::args::OutputFormat::Old {
            if self.run_terminal_timeout(cmd).is_err() {
                VerificationResult::mock_failure()
            } else {
                VerificationResult::mock_success()
            }
        } else {
            // Add extra argument to receive the output in JSON format.
            // Done here because now removed `--visualize` used the XML format instead.
            // TODO: move this now that we don't use --visualize
            cmd.arg("--json-ui");

            self.runtime.block_on(self.run_cbmc_piped(cmd, harness))?
        };

        Ok(verification_results)
    }

    async fn run_cbmc_piped(
        &self,
        mut cmd: TokioCommand,
        harness: &HarnessMetadata,
    ) -> Result<VerificationResult> {
        if self.args.common_args.verbose() {
            println!("[Kani] Running: `{}`", render_command(cmd.as_std()).to_string_lossy());
        }
        // Spawn the CBMC process and process its output below
        let mut cbmc_process = cmd
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| anyhow::Error::msg("Failed to run cbmc"))?;

        let start_time = Instant::now();

        let export_json = self.args.export_json.is_some();
        let (filter, captured_warnings) = warning_capturing_filter(export_json, |item| {
            kani_cbmc_output_filter(
                item,
                self.args.extra_pointer_checks,
                self.args.common_args.quiet,
                &self.args.output_format(),
                self.args.log_file.as_ref(),
            )
        });

        let res = if let Some(timeout) = self.args.harness_timeout {
            tokio::time::timeout(timeout.into(), process_cbmc_output(&mut cbmc_process, filter))
                .await
        } else {
            Ok(process_cbmc_output(&mut cbmc_process, filter).await)
        };

        let mut result = if let Ok(output) = res {
            // The timeout wasn't reached
            VerificationResult::from(output?, harness.attributes.should_panic, start_time)
        } else {
            // An error occurs if the timeout was reached

            // Kill the process
            cbmc_process.kill().await?;

            VerificationResult {
                status: VerificationStatus::Failure,
                failed_properties: FailedProperties::None,
                results: Err(ExitStatus::Timeout),
                runtime: start_time.elapsed(),
                generated_concrete_test: false,
                ignored_quantifiers: 0,
                coverage_results: None,
                warnings: Vec::new(),
                warnings_truncated: 0,
            }
        };
        (result.warnings, result.warnings_truncated) =
            std::mem::take(&mut *captured_warnings.lock().unwrap()).into_parts();
        Ok(result)
    }

    /// "Internal," but also used by call_cbmc_viewer
    pub fn cbmc_flags(
        &self,
        file: &Path,
        harness_metadata: &HarnessMetadata,
    ) -> Result<Vec<OsString>> {
        let mut args = self.cbmc_check_flags();

        if let Some(object_bits) = self.args.cbmc_object_bits() {
            args.push("--object-bits".into());
            args.push(object_bits.to_string().into());
        }

        if let Some(unwind_value) = resolve_unwind_value(&self.args, harness_metadata) {
            args.push("--unwind".into());
            args.push(unwind_value.to_string().into());
        }

        self.handle_solver_args(&harness_metadata.attributes.solver, &mut args)?;

        if self.args.run_sanity_checks {
            args.push("--validate-goto-model".into());
            args.push("--validate-ssa-equation".into());
        }

        if self.args.concrete_playback.is_none() && !self.args.no_slice_formula {
            args.push("--slice-formula".into());
        }

        if self.args.concrete_playback.is_some() {
            args.push("--trace".into());
            // Concrete playback only consumes the values of `kani::any_raw_*`
            // return-value assignments from the trace. CBMC's compact trace
            // retains those (they are regular, non-hidden assignments) while
            // dropping hidden instrumentation steps whose values can dominate
            // the trace by orders of magnitude on contract-heavy harnesses
            // (e.g. 427 MB -> 3 MB of JSON). Requires CBMC with
            // https://github.com/diffblue/cbmc/pull/9135 to have an effect;
            // CBMC versions that do not yet honor `--compact-trace` with
            // `--json-ui` accept but ignore the option, so this is
            // compatible either way.
            args.push("--compact-trace".into());
        }

        args.extend(self.args.cbmc_args.iter().cloned());

        args.push(file.to_owned().into_os_string());

        // Make CBMC verbose by default to tell users about unwinding progress. This should be
        // reviewed as CBMC's verbosity defaults evolve.
        args.push("--verbosity".into());
        args.push("9".into());

        Ok(args)
    }

    /// Just the flags to CBMC that enable property checking of any sort.
    pub fn cbmc_check_flags(&self) -> Vec<OsString> {
        let mut args = Vec::new();

        // We assume that malloc cannot fail, see https://github.com/model-checking/kani/issues/891
        args.push("--no-malloc-may-fail".into());

        // With PR #2630 we generate the appropriate checks directly rather than relying on CBMC's
        // checks (which are for C semantics).
        args.push("--no-undefined-shift-check".into());
        // With PR #647 we use Rust's `-C overflow-checks=on` instead of:
        // --unsigned-overflow-check
        // --signed-overflow-check
        // So these options are deliberately skipped to avoid erroneously re-checking operations.
        args.push("--no-signed-overflow-check".into());

        if !self.args.checks.memory_safety_on() {
            args.push("--no-bounds-check".into());
            args.push("--no-pointer-check".into());
        }
        if self.args.checks.overflow_on() {
            args.push("--nan-check".into());

            // TODO: Implement conversion checks as an optional check.
            // They are a well defined operation in rust, but they may yield unexpected results to
            // many users. https://github.com/model-checking/kani/issues/840
            // We might want to create a transformation pass instead of enabling CBMC since Kani
            // compiler sometimes rely on the bitwise conversion of signed <-> unsigned.
            // args.push("--conversion-check".into());
        } else {
            args.push("--no-div-by-zero-check".into());
        }

        if !self.args.checks.unwinding_on() {
            args.push("--no-unwinding-assertions".into());
        } else {
            args.push("--no-self-loops-to-assumptions".into());
        }

        if self.args.extra_pointer_checks {
            // This was adding a lot of false positives with std dangling pointer. We should
            // still catch any invalid dereference with --pointer-check. Thus, only enable them
            // if the user explicitly request them.
            args.push("--pointer-overflow-check".into());
        } else {
            args.push("--no-pointer-primitive-check".into());
        }

        args
    }

    /// The solver this run will actually use for a harness: `--solver` takes precedence over the
    /// harness attribute, which takes precedence over the default.
    ///
    /// Anything reporting the configuration of a run must resolve it through here rather than
    /// reading the harness attribute directly, or it will describe a different run than the one
    /// `handle_solver_args` builds.
    pub fn resolved_solver<'a>(&'a self, harness_solver: &'a Option<CbmcSolver>) -> &'a CbmcSolver {
        self.args.solver.as_ref().or(harness_solver.as_ref()).unwrap_or(&DEFAULT_SOLVER)
    }

    pub fn handle_solver_args(
        &self,
        harness_solver: &Option<CbmcSolver>,
        args: &mut Vec<OsString>,
    ) -> Result<()> {
        let solver = self.resolved_solver(harness_solver);

        // Check if the specified binary exists in path
        if let CbmcSolver::Binary(solver_binary) = solver
            && which::which(solver_binary).is_err()
        {
            bail!("the specified solver \"{solver_binary}\" was not found in path")
        }
        args.extend(solver_flags(solver));
        Ok(())
    }
}

/// The CBMC flags that select `solver`.
pub(crate) fn solver_flags(solver: &CbmcSolver) -> Vec<OsString> {
    match solver {
        CbmcSolver::Bitwuzla => vec!["--bitwuzla".into()],
        CbmcSolver::Cadical => vec!["--sat-solver".into(), "cadical".into()],
        CbmcSolver::Cvc5 => vec!["--cvc5".into()],
        CbmcSolver::Kissat => vec!["--external-sat-solver".into(), "kissat".into()],
        // Minisat is currently CBMC's default solver, so no need to
        // pass any arguments
        CbmcSolver::Minisat => vec![],
        CbmcSolver::Z3 => vec!["--z3".into()],
        CbmcSolver::Binary(solver_binary) => {
            vec!["--external-sat-solver".into(), solver_binary.into()]
        }
    }
}

/// Count CBMC messages reporting that a quantifier expression could not be encoded and was
/// dropped. CBMC's SAT-based backends only support quantifiers with constant bounds; other
/// quantifiers are replaced by unconstrained values, with only a low-visibility message
/// (`prop_conv_solvert::ignoring`, printed as "warning: ignoring forall" followed by the
/// pretty-printed expression).
fn count_ignored_quantifiers(items: &[ParserItem]) -> usize {
    items
        .iter()
        .filter(|item| {
            matches!(item, ParserItem::Message { message_text, .. }
                if message_text.starts_with("warning: ignoring forall")
                    || message_text.starts_with("warning: ignoring exists"))
        })
        .count()
}

/// The error rendered when the solver backend dropped quantifier expressions.
fn ignored_quantifiers_error(count: usize) -> String {
    format!(
        "error: the solver backend does not support quantifiers with non-constant bounds \
and ignored {count} quantifier expression(s), replacing them with unconstrained values.\n\
         Kani cannot soundly verify this harness: `kani::assume` calls containing such a \
quantifier are NOT enforced (a successful result would not cover the intended property), and \
`kani::assert` calls containing one may fail spuriously.\n\
         Use an SMT solver backend that supports quantifiers, e.g. `#[kani::solver(z3)]`.\n"
    )
}

/// Whether CBMC's exit status shows that it finished reporting its results.
///
/// CBMC exits with 0 when all properties pass and with 10 when some property fails. It exits
/// with 6 when some property has status ERROR (e.g., a solver error), which the results already
/// report as a failure. It exits with 5 when some property is UNKNOWN or NOT_CHECKED; Kani's
/// verdict does not count those as failures, so such results are rejected. Any other status means
/// CBMC failed along the way: for instance, when it runs out of memory while writing a trace, it
/// still closes the JSON output (exit status 6), but the result array is truncated and may be
/// missing failed properties. A truncated array can even contain an ERROR property, so callers
/// check for an out-of-memory message first (see `cbmc_reported_out_of_memory`).
/// See <https://github.com/model-checking/kani/issues/4905>.
fn cbmc_completed_results(process_status: i32, results: &[Property]) -> bool {
    match process_status {
        0 | 10 => true,
        6 => results.iter().any(|prop| prop.status == CheckStatus::Error),
        _ => false,
    }
}

/// Whether CBMC reported that it ran out of memory. CBMC catches `std::bad_alloc`, reports it
/// with this error message, and exits with status 6, a status it shares with other errors.
fn cbmc_reported_out_of_memory(items: &[ParserItem]) -> bool {
    items.iter().any(|item| {
        matches!(item, ParserItem::Message { message_text, message_type }
            if message_type == "ERROR" && message_text == "Out of memory")
    })
}

impl VerificationResult {
    /// Computes a `VerificationResult` (kani-driver's notion of the result of a CBMC call) from a
    /// `VerificationOutput` (cbmc_output_parser's idea of CBMC results).
    ///
    /// NOTE: We mostly ignore the CBMC exit status, in favor of two checks:
    ///   1. Examining the actual results of CBMC properties.
    ///      (CBMC will regularly report "failure" but that's just our cover checks.)
    ///   2. Positively checking for the presence of results.
    ///      (Do not mistake lack of results for success: report it as failure.)
    ///
    /// The exit status is only used to reject results that CBMC did not finish reporting
    /// (see `cbmc_completed_results`).
    pub(crate) fn from(
        output: VerificationOutput,
        should_panic: bool,
        start_time: Instant,
    ) -> VerificationResult {
        let runtime = start_time.elapsed();
        let ignored_quantifiers = count_ignored_quantifiers(&output.processed_items);
        let (remaining_items, results) = extract_results(output.processed_items);

        if let Some(results) = results
            && !cbmc_reported_out_of_memory(&remaining_items)
            && cbmc_completed_results(output.process_status, &results)
        {
            let (mut status, mut failed_properties) =
                verification_outcome_from_properties(&results, should_panic);
            // A dropped quantifier makes the analysis unsound: a `kani::assume` containing one is
            // silently not enforced, so a "successful" result may be vacuous. Kani must never
            // report success in that case -- force a failure and (via the rendered error) direct
            // the user to an SMT backend that supports quantifiers.
            if ignored_quantifiers > 0 {
                status = VerificationStatus::Failure;
                failed_properties = FailedProperties::Error;
            }
            let coverage_results = coverage_results_from_properties(&results);
            VerificationResult {
                status,
                failed_properties,
                results: Ok(results),
                runtime,
                generated_concrete_test: false,
                ignored_quantifiers,
                coverage_results,
                warnings: Vec::new(),
                warnings_truncated: 0,
            }
        } else {
            // We never got (complete) results from CBMC - something went wrong (e.g. crash) so
            // it's failure
            let exit_status =
                if output.process_status == 137 || cbmc_reported_out_of_memory(&remaining_items) {
                    ExitStatus::OutOfMemory
                } else {
                    ExitStatus::Other(output.process_status)
                };
            VerificationResult {
                status: VerificationStatus::Failure,
                failed_properties: FailedProperties::Other,
                results: Err(exit_status),
                runtime,
                generated_concrete_test: false,
                ignored_quantifiers,
                coverage_results: None,
                warnings: Vec::new(),
                warnings_truncated: 0,
            }
        }
    }

    pub fn mock_success() -> VerificationResult {
        VerificationResult {
            status: VerificationStatus::Success,
            failed_properties: FailedProperties::None,
            results: Ok(vec![]),
            runtime: Duration::from_secs(0),
            generated_concrete_test: false,
            ignored_quantifiers: 0,
            coverage_results: None,
            warnings: Vec::new(),
            warnings_truncated: 0,
        }
    }

    fn mock_failure() -> VerificationResult {
        VerificationResult {
            status: VerificationStatus::Failure,
            failed_properties: FailedProperties::Other,
            // on failure, exit codes in theory might be used,
            // but `mock_failure` should never be used in a context where they will,
            // so again use something weird:
            results: Err(ExitStatus::Other(42)),
            runtime: Duration::from_secs(0),
            generated_concrete_test: false,
            ignored_quantifiers: 0,
            coverage_results: None,
            warnings: Vec::new(),
            warnings_truncated: 0,
        }
    }

    pub fn render(&self, output_format: &OutputFormat, should_panic: bool) -> String {
        match &self.results {
            Ok(results) => {
                let status = self.status;
                let failed_properties = self.failed_properties;
                let show_checks = matches!(output_format, OutputFormat::Regular);

                let mut result = if let Some(cov_results) = &self.coverage_results {
                    format_coverage(
                        results,
                        cov_results,
                        status,
                        should_panic,
                        failed_properties,
                        show_checks,
                    )
                } else {
                    format_result(results, status, should_panic, failed_properties, show_checks)
                };
                if self.ignored_quantifiers > 0 {
                    let error = ignored_quantifiers_error(self.ignored_quantifiers);
                    // Surface the soundness error immediately before the overall
                    // `VERIFICATION:- ...` line, so it explains the forced failure. Fall back to
                    // appending it (e.g. coverage output, which has no such line) if the marker
                    // isn't present.
                    match result.find("\nVERIFICATION:- ") {
                        Some(pos) => result.insert_str(pos + 1, &error),
                        None => {
                            result.push('\n');
                            result.push_str(&error);
                        }
                    }
                }
                writeln!(result, "Verification Time: {}s", self.runtime.as_secs_f32()).unwrap();
                result
            }
            Err(exit_status) => {
                let verification_result = console::style("FAILED").red();
                let (header, explanation) = match exit_status {
                    ExitStatus::OutOfMemory => (
                        String::from("CBMC failed"),
                        "CBMC appears to have run out of memory. You may want to rerun your proof in \
                    an environment with additional memory or use stubbing to reduce the size of the \
                    code the verifier reasons about.\n",
                    ),
                    ExitStatus::Timeout => (
                        String::from("CBMC failed"),
                        "CBMC timed out. You may want to rerun your proof with a larger timeout \
                    or use stubbing to reduce the size of the code the verifier reasons about.\n",
                    ),
                    ExitStatus::Other(exit_status) => {
                        (format!("CBMC failed with status {exit_status}"), "")
                    }
                };
                format!(
                    "\n{header}\n\
                    VERIFICATION:- {verification_result}\n\
                    {explanation}",
                )
            }
        }
    }
}

/// We decide if verification succeeded based on properties, not (typically) on exit code
fn verification_outcome_from_properties(
    properties: &[Property],
    should_panic: bool,
) -> (VerificationStatus, FailedProperties) {
    let failed_properties = determine_failed_properties(properties);
    let status = if should_panic {
        match failed_properties {
            FailedProperties::Error => VerificationStatus::Failure,
            FailedProperties::None | FailedProperties::Other => VerificationStatus::Failure,
            FailedProperties::PanicsOnly => VerificationStatus::Success,
        }
    } else {
        match failed_properties {
            FailedProperties::Error => VerificationStatus::Failure,
            FailedProperties::None => VerificationStatus::Success,
            FailedProperties::PanicsOnly | FailedProperties::Other => VerificationStatus::Failure,
        }
    };
    (status, failed_properties)
}

/// Determines the `FailedProperties` variant that corresponds to an array of properties
fn determine_failed_properties(properties: &[Property]) -> FailedProperties {
    if properties.iter().any(|prop| prop.status == CheckStatus::Error) {
        return FailedProperties::Error;
    };
    let failed_properties: Vec<&Property> =
        properties.iter().filter(|prop| prop.status == CheckStatus::Failure).collect();
    // Return `FAILURE` if there isn't at least one failed property
    if failed_properties.is_empty() {
        FailedProperties::None
    } else {
        // Check if all failed properties correspond to the `assertion` class.
        // Note: Panics caused by `panic!` and `assert!` fall into this class.
        let all_failed_checks_are_panics =
            failed_properties.iter().all(|prop| prop.property_class() == "assertion");
        if all_failed_checks_are_panics {
            FailedProperties::PanicsOnly
        } else {
            FailedProperties::Other
        }
    }
}

fn coverage_results_from_properties(properties: &[Property]) -> Option<CoverageResults> {
    let cov_properties: Vec<&Property> =
        properties.iter().filter(|p| p.is_code_coverage_property()).collect();

    if cov_properties.is_empty() {
        return None;
    }

    // Postprocessing the coverage results involves matching on the descriptions
    // of code coverage properties with the `counter_re` regex. These are two
    // real examples of such descriptions:
    //
    // ```
    // CounterIncrement(0) $test_cov$ - src/main.rs:5:1 - 6:15
    // ExpressionUsed(0) $test_cov$ - src/main.rs:6:19 - 6:28
    // ```
    //
    // The span is further processed to extract the code region attributes.
    // Ideally, we should have coverage mappings (i.e., the relation between
    // counters and code regions) available in the coverage metadata:
    // <https://github.com/model-checking/kani/issues/3445>. If that were the
    // case, we would not need the spans in these descriptions.
    let counter_re = {
        static COUNTER_RE: OnceLock<Regex> = OnceLock::new();
        COUNTER_RE.get_or_init(|| {
            Regex::new(
                r#"^(?<kind>VirtualCounter\(bcb)(?<counter_num>[0-9]+)\) \$(?<func_name>[^\$]+)\$ - (?<span>.+)"#,
            )
            .unwrap()
        })
    };

    let mut coverage_results: BTreeMap<String, Vec<CoverageCheck>> = BTreeMap::default();

    for prop in cov_properties {
        let mut prop_processed = false;
        if let Some(captures) = counter_re.captures(&prop.description) {
            let counter_num = &captures["counter_num"];
            let function = demangle(&captures["func_name"]).to_string();
            let status = prop.status;
            let span = captures["span"].to_string();

            let counter_id = counter_num.parse().unwrap();
            let term = CoverageTerm::Counter(counter_id);
            let region = CoverageRegion::from_str(span);

            let cov_check = CoverageCheck::new(function, term, region, status);
            let file = cov_check.region.file.clone();

            if let Entry::Vacant(e) = coverage_results.entry(file.clone()) {
                e.insert(vec![cov_check]);
            } else {
                coverage_results.entry(file).and_modify(|checks| checks.push(cov_check));
            }
            prop_processed = true;
        }

        assert!(prop_processed, "error: coverage property not processed\n{prop:?}");
    }

    Some(CoverageResults::new(coverage_results))
}
/// Solve Unwind Value from conflicting inputs of unwind values. (--default-unwind, annotation-unwind, --unwind)
pub fn resolve_unwind_value(
    args: &VerificationArgs,
    harness_metadata: &HarnessMetadata,
) -> Option<u32> {
    // Check for which flag is being passed and prioritize extracting unwind from the
    // respective flag/annotation.
    args.unwind.or(harness_metadata.attributes.unwind_value).or(args.default_unwind)
}

#[cfg(test)]
mod tests {
    use crate::args;
    use crate::cbmc_output_parser::{PropertyId, SourceLocation};
    use crate::metadata::tests::mock_proof_harness;
    use clap::Parser;

    use super::*;

    fn property(class: &str, id: u32, status: CheckStatus) -> Property {
        Property {
            description: format!("{class} check"),
            property_id: PropertyId {
                fn_name: Some("harness".to_string()),
                class: class.to_string(),
                id,
            },
            source_location: SourceLocation {
                file: Some("src/lib.rs".to_string()),
                line: Some("12".to_string()),
                column: Some("3".to_string()),
                function: Some("harness".to_string()),
            },
            status,
            reach: None,
            trace: None,
        }
    }

    #[test]
    fn determine_failed_properties_classifies_by_status_and_class() {
        assert!(matches!(
            determine_failed_properties(&[property("assertion", 1, CheckStatus::Success)]),
            FailedProperties::None
        ));
        assert!(matches!(
            determine_failed_properties(&[
                property("assertion", 1, CheckStatus::Failure),
                property("assertion", 2, CheckStatus::Success),
            ]),
            FailedProperties::PanicsOnly
        ));
        assert!(matches!(
            determine_failed_properties(&[
                property("assertion", 1, CheckStatus::Failure),
                property("overflow", 2, CheckStatus::Failure),
            ]),
            FailedProperties::Other
        ));
        assert!(matches!(
            determine_failed_properties(&[property("overflow", 1, CheckStatus::Failure)]),
            FailedProperties::Other
        ));
        assert!(matches!(
            determine_failed_properties(&[
                property("assertion", 1, CheckStatus::Failure),
                property("cover", 2, CheckStatus::Error),
            ]),
            FailedProperties::Error
        ));
    }

    #[test]
    fn verification_outcome_from_properties_truth_table() {
        let none = [property("assertion", 1, CheckStatus::Success)];
        let panics_only = [property("assertion", 1, CheckStatus::Failure)];
        let other = [property("overflow", 1, CheckStatus::Failure)];
        let error = [property("cover", 1, CheckStatus::Error)];

        assert!(matches!(
            verification_outcome_from_properties(&none, false),
            (VerificationStatus::Success, FailedProperties::None)
        ));
        assert!(matches!(
            verification_outcome_from_properties(&panics_only, false),
            (VerificationStatus::Failure, FailedProperties::PanicsOnly)
        ));
        assert!(matches!(
            verification_outcome_from_properties(&other, false),
            (VerificationStatus::Failure, FailedProperties::Other)
        ));
        assert!(matches!(
            verification_outcome_from_properties(&error, false),
            (VerificationStatus::Failure, FailedProperties::Error)
        ));

        assert!(matches!(
            verification_outcome_from_properties(&none, true),
            (VerificationStatus::Failure, FailedProperties::None)
        ));
        assert!(matches!(
            verification_outcome_from_properties(&panics_only, true),
            (VerificationStatus::Success, FailedProperties::PanicsOnly)
        ));
        assert!(matches!(
            verification_outcome_from_properties(&other, true),
            (VerificationStatus::Failure, FailedProperties::Other)
        ));
        assert!(matches!(
            verification_outcome_from_properties(&error, true),
            (VerificationStatus::Failure, FailedProperties::Error)
        ));
    }

    #[test]
    fn verification_result_from_ignored_quantifiers_overrides_without_error_status_property() {
        let output = VerificationOutput {
            process_status: 0,
            processed_items: vec![
                ParserItem::Message {
                    message_text: "warning: ignoring forall (x : int) ...".to_string(),
                    message_type: "WARNING".to_string(),
                },
                ParserItem::Result { result: vec![property("assertion", 1, CheckStatus::Success)] },
            ],
        };
        let vr = VerificationResult::from(output, false, Instant::now());
        assert_eq!(vr.ignored_quantifiers, 1);
        assert!(matches!(vr.status, VerificationStatus::Failure));
        assert!(matches!(vr.failed_properties, FailedProperties::Error));
    }

    #[test]
    fn verification_result_from_without_ignored_quantifiers_does_not_override() {
        let output = VerificationOutput {
            process_status: 0,
            processed_items: vec![ParserItem::Result {
                result: vec![property("assertion", 1, CheckStatus::Success)],
            }],
        };
        let vr = VerificationResult::from(output, false, Instant::now());
        assert_eq!(vr.ignored_quantifiers, 0);
        assert!(matches!(vr.status, VerificationStatus::Success));
        assert!(matches!(vr.failed_properties, FailedProperties::None));
    }

    #[test]
    fn verification_result_from_no_results_reports_out_of_memory_for_process_status_137() {
        let output = VerificationOutput {
            process_status: 137,
            processed_items: vec![ParserItem::Message {
                message_text: "Killed".to_string(),
                message_type: "STATUS-MESSAGE".to_string(),
            }],
        };
        let vr = VerificationResult::from(output, false, Instant::now());
        assert!(matches!(vr.status, VerificationStatus::Failure));
        assert!(matches!(vr.failed_properties, FailedProperties::Other));
        assert!(matches!(vr.results, Err(ExitStatus::OutOfMemory)));
    }

    #[test]
    fn verification_result_from_no_results_reports_other_for_non_137_process_status() {
        let output = VerificationOutput { process_status: 6, processed_items: vec![] };
        let vr = VerificationResult::from(output, false, Instant::now());
        assert!(matches!(vr.status, VerificationStatus::Failure));
        assert!(matches!(vr.failed_properties, FailedProperties::Other));
        assert!(matches!(vr.results, Err(ExitStatus::Other(6))));
    }

    #[test]
    fn warning_message_truncation_boundary() {
        let cases: [(i64, bool); 4] = [(-1, false), (0, false), (1, true), (500, true)];
        for (len_delta, expect_truncated) in cases {
            let len = (MAX_WARNING_MESSAGE_CHARS as i64 + len_delta) as usize;
            let message = "x".repeat(len);
            let w = truncate_warning(&message);
            assert_eq!(w.truncated, expect_truncated, "len_delta={len_delta}");
            if expect_truncated {
                assert_eq!(w.message, "x".repeat(MAX_WARNING_MESSAGE_CHARS));
                assert_eq!(w.original_chars, Some(len));
            } else {
                assert_eq!(w.message, message);
                assert_eq!(w.original_chars, None);
            }
        }
    }

    #[test]
    fn captured_warnings_cap_boundary() {
        for (extra, expect_truncated) in [(0, 0), (1, 1), (3, 3)] {
            let mut captured = CapturedWarnings::default();
            for i in 0..MAX_WARNINGS_PER_HARNESS + extra {
                captured.push(&format!("warning {i}"));
            }
            let (warnings, truncated) = captured.into_parts();
            assert_eq!(warnings.len(), MAX_WARNINGS_PER_HARNESS, "extra={extra}");
            assert_eq!(truncated, expect_truncated, "extra={extra}");
            assert_eq!(warnings[0].message, "warning 0");
        }
    }

    #[test]
    fn warning_capturing_filter_ignores_warnings_when_export_json_disabled() {
        let (mut filter, captured) = warning_capturing_filter(false, Some);
        let _ = filter(ParserItem::Message {
            message_text: "warning 0".to_string(),
            message_type: "WARNING".to_string(),
        });
        let (warnings, truncated) = std::mem::take(&mut *captured.lock().unwrap()).into_parts();
        assert!(warnings.is_empty());
        assert_eq!(truncated, 0);
    }

    #[test]
    fn warning_capturing_filter_only_captures_warning_typed_messages() {
        let (mut filter, captured) = warning_capturing_filter(true, Some);
        let _ = filter(ParserItem::Message {
            message_text: "not a warning".to_string(),
            message_type: "STATUS-MESSAGE".to_string(),
        });
        let _ = filter(ParserItem::Message {
            message_text: "warning 0".to_string(),
            message_type: "WARNING".to_string(),
        });
        let _ = filter(ParserItem::Message {
            message_text: "warning 1".to_string(),
            message_type: "warning".to_string(),
        });
        let (warnings, _) = std::mem::take(&mut *captured.lock().unwrap()).into_parts();
        let messages: Vec<&str> = warnings.iter().map(|w| w.message.as_str()).collect();
        assert_eq!(messages, ["warning 0", "warning 1"]);
    }

    #[test]
    fn warning_capture_survives_a_real_timeout_cancellation() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let total = MAX_WARNINGS_PER_HARNESS + 5;
            let mut script = String::from(
                "printf '[\\n  {\\n    \"program\": \"unit-test fake cbmc\"\\n  },\\n'; ",
            );
            for i in 0..total {
                script.push_str(&format!(
                    "printf '  {{\\n    \"messageText\": \"warning {i}\",\\n    \"messageType\": \"WARNING\"\\n  }},\\n'; "
                ));
            }
            script.push_str("exec sleep 100");

            let mut child = TokioCommand::new("sh")
                .arg("-c")
                .arg(script)
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap();

            let (filter, captured) = warning_capturing_filter(true, Some);
            let mut parse = Box::pin(process_cbmc_output(&mut child, filter));
            let all_seen = tokio::time::timeout(
                Duration::from_secs(30),
                std::future::poll_fn(|cx| {
                    if parse.as_mut().poll(cx).is_ready() {
                        return std::task::Poll::Ready(false);
                    }
                    let captured = captured.lock().unwrap();
                    if captured.warnings.len() + captured.truncated == total {
                        std::task::Poll::Ready(true)
                    } else {
                        std::task::Poll::Pending
                    }
                }),
            )
            .await;
            assert_eq!(all_seen.ok(), Some(true), "the fake process must emit every warning and stay blocked");
            drop(parse);
            child.kill().await.unwrap();

            let (warnings, truncated) =
                std::mem::take(&mut *captured.lock().unwrap()).into_parts();
            assert_eq!(warnings.len(), MAX_WARNINGS_PER_HARNESS);
            assert_eq!(truncated, 5);
            assert_eq!(warnings[0].message, "warning 0");
        });
    }

    #[test]
    fn solver_flags_per_solver() {
        let flags = |solver: CbmcSolver| -> Vec<String> {
            solver_flags(&solver).iter().map(|f| f.to_string_lossy().into_owned()).collect()
        };
        assert_eq!(flags(CbmcSolver::Bitwuzla), ["--bitwuzla"]);
        assert_eq!(flags(CbmcSolver::Cadical), ["--sat-solver", "cadical"]);
        assert_eq!(flags(CbmcSolver::Cvc5), ["--cvc5"]);
        assert_eq!(flags(CbmcSolver::Kissat), ["--external-sat-solver", "kissat"]);
        assert!(flags(CbmcSolver::Minisat).is_empty());
        assert_eq!(flags(CbmcSolver::Z3), ["--z3"]);
        assert_eq!(
            flags(CbmcSolver::Binary("my-solver".to_string())),
            ["--external-sat-solver", "my-solver"]
        );
    }

    /// The output of a CBMC run that reported a single property with the given status, followed
    /// by the given error messages, and exited with `process_status`.
    fn mock_cbmc_output(
        process_status: i32,
        property_status: &str,
        errors: &[&str],
    ) -> VerificationOutput {
        let result = format!(
            r#"{{
                "result": [
                    {{
                        "description": "assertion failed: x < 10",
                        "property": "check.assertion.1",
                        "sourceLocation": {{ "file": "main.rs", "function": "check", "line": "5" }},
                        "status": "{property_status}"
                    }}
                ]
            }}"#
        );
        let mut processed_items = vec![serde_json::from_str(&result).unwrap()];
        processed_items.extend(errors.iter().map(|text| ParserItem::Message {
            message_text: text.to_string(),
            message_type: "ERROR".to_string(),
        }));
        VerificationOutput { process_status, processed_items }
    }

    fn verify(output: VerificationOutput) -> VerificationResult {
        VerificationResult::from(output, false, Instant::now())
    }

    /// CBMC's exit status for complete results must not change the verdict the properties give.
    #[test]
    fn check_complete_results_decide_verdict() {
        let result = verify(mock_cbmc_output(0, "SUCCESS", &[]));
        assert_eq!(result.status, VerificationStatus::Success);
        assert!(result.results.is_ok());

        let result = verify(mock_cbmc_output(10, "FAILURE", &[]));
        assert_eq!(result.status, VerificationStatus::Failure);
        assert!(result.results.is_ok());

        // CBMC exits with status 6 when a property has status ERROR, e.g. on a solver error.
        let result = verify(mock_cbmc_output(6, "ERROR", &[]));
        assert_eq!(result.status, VerificationStatus::Failure);
        assert!(matches!(result.failed_properties, FailedProperties::Error));
        assert!(result.results.is_ok());
    }

    /// When CBMC runs out of memory while writing its results, the result array it emitted is
    /// truncated and must not be mistaken for success.
    /// See <https://github.com/model-checking/kani/issues/4905>.
    #[test]
    fn check_truncated_results_fail() {
        let result = verify(mock_cbmc_output(6, "SUCCESS", &["Out of memory"]));
        assert_eq!(result.status, VerificationStatus::Failure);
        assert!(matches!(result.results, Err(ExitStatus::OutOfMemory)));

        let result = verify(mock_cbmc_output(6, "SUCCESS", &["some internal error"]));
        assert_eq!(result.status, VerificationStatus::Failure);
        assert!(matches!(result.results, Err(ExitStatus::Other(6))));

        // A truncated array can contain an ERROR property from before the truncation point.
        let result = verify(mock_cbmc_output(6, "ERROR", &["Out of memory"]));
        assert_eq!(result.status, VerificationStatus::Failure);
        assert!(matches!(result.results, Err(ExitStatus::OutOfMemory)));

        // Killed by a signal (here SIGSEGV) after writing some results.
        let result = verify(mock_cbmc_output(139, "SUCCESS", &[]));
        assert_eq!(result.status, VerificationStatus::Failure);
        assert!(matches!(result.results, Err(ExitStatus::Other(139))));

        // CBMC exits with 5 when some property is UNKNOWN or NOT_CHECKED. Kani's verdict logic
        // does not treat UNKNOWN as a failure, so these results must not be used.
        let result = verify(mock_cbmc_output(5, "UNKNOWN", &[]));
        assert_eq!(result.status, VerificationStatus::Failure);
        assert!(matches!(result.results, Err(ExitStatus::Other(5))));
    }

    #[test]
    fn check_resolve_unwind_value() {
        // Command line unwind value for specific harnesses take precedence over default annotation value
        let args_empty = ["kani", "x.rs"];
        let args_only_default = ["kani", "x.rs", "--default-unwind", "2"];
        let args_only_harness = ["kani", "x.rs", "--unwind", "1", "--harness", "check_one"];
        let args_both =
            ["kani", "x.rs", "--default-unwind", "2", "--unwind", "1", "--harness", "check_one"];

        let harness_none = mock_proof_harness("check_one", None, None, None);
        let harness_some = mock_proof_harness("check_one", Some(3), None, None);

        fn resolve(args: &[&str], harness: &HarnessMetadata) -> Option<u32> {
            resolve_unwind_value(
                &args::StandaloneArgs::try_parse_from(args).unwrap().verify_opts,
                harness,
            )
        }

        // test against no unwind annotation
        assert_eq!(resolve(&args_empty, &harness_none), None);
        assert_eq!(resolve(&args_only_default, &harness_none), Some(2));
        assert_eq!(resolve(&args_only_harness, &harness_none), Some(1));
        assert_eq!(resolve(&args_both, &harness_none), Some(1));

        // test against unwind annotation
        assert_eq!(resolve(&args_empty, &harness_some), Some(3));
        assert_eq!(resolve(&args_only_default, &harness_some), Some(3));
        assert_eq!(resolve(&args_only_harness, &harness_some), Some(1));
        assert_eq!(resolve(&args_both, &harness_some), Some(1));
    }
}
