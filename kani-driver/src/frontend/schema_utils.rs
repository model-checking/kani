// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! `--export-json`: one JSON file per verification run, per RFC 0015 (Kani issue #942).

use crate::args::VerificationArgs;
use crate::call_cbmc::{
    ExitStatus, FailedProperties, VerificationStatus, Warning, resolve_unwind_value, solver_flags,
};
use crate::cbmc_output_parser::{CheckStatus, Property};
use crate::harness_runner::HarnessResult;
use crate::sarif::relativize_path;
use crate::session::KaniSession;
use crate::version::{KANI_RUSTC_RELEASE, KANI_VERSION, cbmc_version_on_path};
use anyhow::{Context, Result};
use kani_metadata::{
    AssignsContract, CbmcSolver, HarnessAttributes, HarnessMetadata, find_proof_harnesses,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;
use tempfile::Builder as TempFileBuilder;
use time::OffsetDateTime;
use time::format_description;

const SCHEMA_VERSION: &str = "0.1.0";

const KANI_GIT_SHA: Option<&str> = option_env!("KANI_GIT_SHA");
/// `None` exactly when `KANI_GIT_SHA` is `None`.
const KANI_GIT_DIRTY: Option<&str> = option_env!("KANI_GIT_DIRTY");

struct RunContext {
    cbmc_version: Option<String>,
    rustc_version: Option<String>,
    kani_commit: Option<&'static str>,
    kani_commit_dirty: Option<bool>,
    enabled_unstable_features: Vec<String>,
    harness_selection: HarnessSelectionExport,
    harness_timeout_s: Option<f64>,
    configuration: ConfigurationExport,
    started_at: OffsetDateTime,
    wall_time: Duration,
}

impl KaniSession {
    /// Writes `--export-json` for a completed run; no-op if unset. `matched_harnesses` must be
    /// the pre-verification list, not `results`, which `--fail-fast` can truncate.
    pub fn write_export_json(
        &self,
        matched_harnesses: &[&HarnessMetadata],
        results: &[HarnessResult<'_>],
        started_at: OffsetDateTime,
        wall_time: Duration,
    ) -> Result<()> {
        let Some(path) = &self.args.export_json else { return Ok(()) };
        let ctx = self.build_run_context(matched_harnesses, started_at, wall_time);
        let export = ExportedRun::from_harness_results(
            results,
            |harness| {
                let base = self.resolved_solver(&harness.attributes.solver);
                (
                    effective_solver(&self.args.cbmc_args, base),
                    resolve_effective_unwind(&self.args, harness, &self.args.cbmc_args),
                )
            },
            ctx,
        );
        write_json_atomically(path, &export)
    }

    fn build_run_context(
        &self,
        matched_harnesses: &[&HarnessMetadata],
        started_at: OffsetDateTime,
        wall_time: Duration,
    ) -> RunContext {
        let enabled_unstable_features: Vec<String> = self
            .args
            .common_args
            .unstable_features
            .iter()
            .map(|feature| feature.as_ref().to_string())
            .collect();

        RunContext {
            cbmc_version: cbmc_version_on_path(),
            rustc_version: build_rustc_release(),
            kani_commit: KANI_GIT_SHA,
            kani_commit_dirty: KANI_GIT_DIRTY.map(|dirty| dirty == "true"),
            enabled_unstable_features,
            harness_selection: HarnessSelectionExport {
                requested_filters: self.args.harnesses.clone(),
                exact: self.args.exact,
                unmatched_filters: compute_unmatched_filters(
                    &self.args.harnesses,
                    matched_harnesses,
                    self.args.exact,
                ),
                matched_count: matched_harnesses.len(),
            },
            harness_timeout_s: self.args.harness_timeout.map(|t| Duration::from(t).as_secs_f64()),
            configuration: configuration_from(&self.args),
            started_at,
            wall_time,
        }
    }
}

fn checks_flags_from(args: &VerificationArgs) -> ChecksFlags {
    ChecksFlags {
        memory_safety: args.checks.memory_safety_on(),
        overflow: args.checks.overflow_on(),
        unwinding: args.checks.unwinding_on(),
        undefined_function: args.checks.undefined_function_on(),
        assertion_reach_checks: args.assertion_reach_checks(),
        ignore_global_asm: args.ignore_global_asm,
        extra_pointer_checks: args.extra_pointer_checks,
        assert_contracts: !args.no_assert_contracts,
        prove_safety_only: args.prove_safety_only,
    }
}

fn configuration_from(args: &VerificationArgs) -> ConfigurationExport {
    ConfigurationExport {
        checks: checks_flags_from(args),
        cbmc_args: args.cbmc_args.iter().map(|s| s.to_string_lossy().into_owned()).collect(),
        coverage_enabled: args.coverage,
    }
}

fn compute_unmatched_filters(
    requested: &[String],
    matched_harnesses: &[&HarnessMetadata],
    exact: bool,
) -> Vec<String> {
    requested
        .iter()
        .filter(|filter| {
            let targets: BTreeSet<&String> = BTreeSet::from([*filter]);
            find_proof_harnesses(&targets, matched_harnesses.iter().copied(), exact).is_empty()
        })
        .cloned()
        .collect()
}

/// Writes `value` as pretty JSON to a temporary file in `path`'s directory, then renames it onto
/// `path`, so a reader never sees a partial file and a failed write leaves any earlier file intact.
fn write_json_atomically<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create --export-json output directory `{}`", parent.display())
        })?;
    }

    let dir = export_parent_dir(path);
    let mut builder = TempFileBuilder::new();
    // Short fixed prefix, not the destination's name: a near-255-byte destination plus
    // `NamedTempFile`'s random suffix would overflow the filesystem's per-component limit.
    builder.prefix(".kani-export-");
    // Give the file the mode `std::fs::write` would: an existing target keeps its mode, a new one
    // gets 0o666 minus the umask (`NamedTempFile` defaults to 0o600).
    #[cfg(unix)]
    let existing_permissions =
        std::fs::metadata(path).ok().filter(|m| m.is_file()).map(|m| m.permissions());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o666));
    }
    let mut tmp = builder.tempfile_in(dir).with_context(|| {
        format!("Failed to create --export-json temporary file in `{}`", dir.display())
    })?;
    let tmp_path = tmp.path().to_path_buf();
    let write_error = || {
        format!(
            "Failed to write --export-json output to `{}` (temporary file `{}`)",
            path.display(),
            tmp_path.display()
        )
    };
    #[cfg(unix)]
    if let Some(permissions) = existing_permissions {
        tmp.as_file().set_permissions(permissions).with_context(write_error)?;
    }
    {
        let mut writer = BufWriter::new(&mut tmp);
        serde_json::to_writer_pretty(&mut writer, value).with_context(write_error)?;
        writer.write_all(b"\n").with_context(write_error)?;
        writer.flush().with_context(write_error)?;
    }
    tmp.as_file().sync_all().with_context(write_error)?;
    tmp.persist(path).map_err(|e| e.error).with_context(|| {
        format!("Failed to persist --export-json temporary file to `{}`", path.display())
    })?;
    Ok(())
}

fn export_parent_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// The rustc release `kani-compiler` was built with, e.g. `1.98.0-nightly (14210df0e 2026-05-31)`.
fn build_rustc_release() -> Option<String> {
    (!KANI_RUSTC_RELEASE.is_empty()).then(|| KANI_RUSTC_RELEASE.to_string())
}

/// CBMC's SMT backends, highest priority first. CBMC picks the first one present, whatever the
/// argument order: `--cvc5 --z3` and `--z3 --cvc5` both run CVC 5.
const SMT_SOLVER_PRIORITY: [(&str, &str); 7] = [
    ("--bitwuzla", "bitwuzla"),
    ("--boolector", "boolector"),
    ("--cprover-smt2", "cprover-smt2"),
    ("--mathsat", "mathsat"),
    ("--cvc5", "cvc5"),
    ("--yices", "yices"),
    ("--z3", "z3"),
];

/// The solver CBMC runs for Kani's own solver flags followed by `--cbmc-args`, or `None` when
/// CBMC chooses for itself (bare `--smt2`). CBMC uses the first occurrence of a repeated
/// `--sat-solver`, `--external-sat-solver`, `--external-smt2-solver` or `--unwind`, so a
/// `--cbmc-args --sat-solver` does not replace the one Kani passes. A named SMT solver flag keeps
/// its dialect while `--external-smt2-solver` swaps in another executable, which is what runs.
/// CBMC rejects an SMT flag together with `--external-sat-solver`, or a named one with
/// `--incremental-smt2-solver`; the harness then does not complete, so the result is best effort.
fn effective_solver(cbmc_args: &[OsString], base: &CbmcSolver) -> Option<String> {
    let argv: Vec<OsString> =
        solver_flags(base).into_iter().chain(cbmc_args.iter().cloned()).collect();
    let argv: Vec<Option<&str>> = argv.iter().map(|arg| arg.to_str()).collect();
    let has = |flag: &str| argv.contains(&Some(flag));
    let value = |flag: &str| {
        let position = argv.iter().position(|arg| *arg == Some(flag))?;
        argv.get(position + 1).copied().flatten().map(str::to_string)
    };

    if let Some((_, name)) = SMT_SOLVER_PRIORITY.iter().find(|(flag, _)| has(flag)) {
        return Some(value("--external-smt2-solver").unwrap_or_else(|| name.to_string()));
    }
    if let Some(executable) =
        value("--incremental-smt2-solver").or_else(|| value("--external-smt2-solver"))
    {
        return Some(executable);
    }
    if has("--smt2") {
        return None;
    }
    // No solver flag at all is CBMC's own default.
    Some(
        value("--external-sat-solver")
            .or_else(|| value("--sat-solver"))
            .unwrap_or_else(|| "minisat".to_string()),
    )
}

/// `--unwind`'s effective bound. Kani's own bound (CLI, harness attribute or `--default-unwind`)
/// precedes `--cbmc-args` on CBMC's command line and CBMC takes the first occurrence, so a raw
/// `--cbmc-args --unwind` only applies when Kani passes none.
fn resolve_effective_unwind(
    args: &VerificationArgs,
    harness: &HarnessMetadata,
    cbmc_args: &[OsString],
) -> Option<u32> {
    if let Some(native) = resolve_unwind_value(args, harness) {
        return Some(native);
    }
    let mut cbmc_args = cbmc_args.iter();
    while let Some(arg) = cbmc_args.next() {
        if arg.to_str() == Some("--unwind") {
            return cbmc_args.next().and_then(|v| v.to_str()).and_then(|v| v.parse().ok());
        }
    }
    None
}

#[derive(Serialize)]
struct ToolsExport {
    kani: &'static str,
    rustc: Option<String>,
    cbmc: Option<String>,
}

#[derive(Serialize)]
struct ExportedRun {
    schema_version: &'static str,
    kani_commit: Option<&'static str>,
    kani_commit_dirty: Option<bool>,
    tools: ToolsExport,
    enabled_unstable_features: Vec<String>,
    harness_selection: HarnessSelectionExport,
    harness_timeout_s: Option<f64>,
    configuration: ConfigurationExport,
    outcome: RunOutcome,
    run_state: RunState,
    target: &'static str,
    started_at: String,
    wall_time_s: f64,
    harnesses: Vec<HarnessExport>,
    summary: Summary,
}

#[derive(Serialize)]
struct HarnessSelectionExport {
    requested_filters: Vec<String>,
    exact: bool,
    unmatched_filters: Vec<String>,
    matched_count: usize,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum RunState {
    Complete,
    NoHarnessesSelected,
    Partial,
}

#[derive(Serialize)]
struct ConfigurationExport {
    checks: ChecksFlags,
    cbmc_args: Vec<String>,
    coverage_enabled: bool,
}

#[derive(Serialize)]
struct ChecksFlags {
    memory_safety: bool,
    overflow: bool,
    unwinding: bool,
    undefined_function: bool,
    assertion_reach_checks: bool,
    ignore_global_asm: bool,
    extra_pointer_checks: bool,
    assert_contracts: bool,
    prove_safety_only: bool,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Verdict {
    Failure,
    Success,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
enum Outcome {
    Completed { verdict: Verdict },
    Crashed { code: Option<i32>, message: Option<String> },
    OutOfMemory,
    Timeout,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
enum RunOutcome {
    Completed,
}

impl From<&ExitStatus> for Outcome {
    fn from(status: &ExitStatus) -> Self {
        match status {
            ExitStatus::Timeout => Outcome::Timeout,
            ExitStatus::OutOfMemory => Outcome::OutOfMemory,
            ExitStatus::Other(code) => Outcome::Crashed {
                code: Some(*code),
                message: Some(format!("CBMC failed with status {code}")),
            },
        }
    }
}

fn is_successful(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::Completed { verdict: Verdict::Success })
}

fn is_completed_failure(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::Completed { verdict: Verdict::Failure })
}

#[derive(Serialize)]
struct HarnessExport {
    name: String,
    crate_name: String,
    file: String,
    line: usize,
    contract: Option<AssignsContract>,
    is_automatically_generated: bool,
    has_loop_contracts: bool,
    is_bounded: bool,
    attributes: HarnessAttributes,
    outcome: Outcome,
    resolved_solver: Option<String>,
    resolved_unwind: Option<u32>,
    generated_concrete_test: bool,
    resources: ResourcesExport,
    n_properties: Option<usize>,
    n_failed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_kind: Option<FailedProperties>,
    failed_properties: Vec<PropertyExport>,
    unsupported_constructs: Vec<PropertyExport>,
    warnings: Vec<Warning>,
    warnings_truncated: usize,
    checks: ChecksExport,
    covers: CoversExport,
}

#[derive(Serialize)]
struct ResourcesExport {
    verification_time_s: f64,
}

#[derive(Serialize)]
struct PropertyExport {
    id: String,
    description: String,
    class: String,
    file: Option<String>,
    line: Option<String>,
    trace_available: bool,
    status: CheckStatus,
}

impl PropertyExport {
    fn from_property(p: &Property) -> Self {
        PropertyExport {
            id: p.property_id.to_cbmc_id(),
            description: p.description.clone(),
            class: p.property_class(),
            file: p.source_location.file.clone(),
            line: p.source_location.line.clone(),
            trace_available: p.trace.is_some(),
            status: p.status,
        }
    }
}

#[derive(Serialize)]
struct OtherPropertyExport {
    id: String,
    status: CheckStatus,
}

#[derive(Serialize)]
struct ChecksExport {
    total: Option<usize>,
    success: Option<usize>,
    failure: Vec<String>,
    unreachable: Vec<String>,
    undetermined: Vec<String>,
    error: Vec<String>,
    unknown: Vec<String>,
    other: Vec<OtherPropertyExport>,
}

impl ChecksExport {
    fn from_properties(properties: &[Property]) -> Self {
        let checks: Vec<&Property> = properties
            .iter()
            .filter(|p| !p.is_cover_property() && !p.is_code_coverage_property())
            .collect();
        let b = bucket_by_status(&checks, CheckStatus::Success, CheckStatus::Failure);
        ChecksExport {
            total: Some(b.total),
            success: Some(b.good.len()),
            failure: b.bad,
            unreachable: b.unreachable,
            undetermined: b.undetermined,
            error: b.error,
            unknown: b.unknown,
            other: b.other,
        }
    }

    fn empty_not_completed() -> Self {
        ChecksExport { total: None, success: None, ..Self::from_properties(&[]) }
    }
}

#[derive(Serialize)]
struct CoversExport {
    total: Option<usize>,
    satisfied: Vec<String>,
    unsatisfiable: Vec<String>,
    unreachable: Vec<String>,
    undetermined: Vec<String>,
    error: Vec<String>,
    unknown: Vec<String>,
    other: Vec<OtherPropertyExport>,
}

impl CoversExport {
    fn from_properties(properties: &[Property]) -> Self {
        let covers: Vec<&Property> = properties.iter().filter(|p| p.is_cover_property()).collect();
        let b = bucket_by_status(&covers, CheckStatus::Satisfied, CheckStatus::Unsatisfiable);
        CoversExport {
            total: Some(b.total),
            satisfied: b.good,
            unsatisfiable: b.bad,
            unreachable: b.unreachable,
            undetermined: b.undetermined,
            error: b.error,
            unknown: b.unknown,
            other: b.other,
        }
    }

    fn empty_not_completed() -> Self {
        CoversExport { total: None, ..Self::from_properties(&[]) }
    }
}

struct StatusBuckets {
    total: usize,
    good: Vec<String>,
    bad: Vec<String>,
    unreachable: Vec<String>,
    undetermined: Vec<String>,
    error: Vec<String>,
    unknown: Vec<String>,
    other: Vec<OtherPropertyExport>,
}

fn bucket_by_status(
    properties: &[&Property],
    good_status: CheckStatus,
    bad_status: CheckStatus,
) -> StatusBuckets {
    let total = properties.len();
    let mut good = Vec::new();
    let mut bad = Vec::new();
    let mut unreachable = Vec::new();
    let mut undetermined = Vec::new();
    let mut error = Vec::new();
    let mut unknown = Vec::new();
    let mut other = Vec::new();

    for p in properties {
        let id = p.property_id.to_cbmc_id();
        let status = p.status;
        if status == good_status {
            good.push(id);
        } else if status == bad_status {
            bad.push(id);
        } else {
            match status {
                CheckStatus::Unreachable => unreachable.push(id),
                CheckStatus::Undetermined => undetermined.push(id),
                CheckStatus::Error => error.push(id),
                CheckStatus::Unknown => unknown.push(id),
                CheckStatus::Covered
                | CheckStatus::Failure
                | CheckStatus::Satisfied
                | CheckStatus::Success
                | CheckStatus::Uncovered
                | CheckStatus::Unsatisfiable => other.push(OtherPropertyExport { id, status }),
            }
        }
    }

    StatusBuckets { total, good, bad, unreachable, undetermined, error, unknown, other }
}

#[derive(Serialize)]
struct Summary {
    total: usize,
    successful: usize,
    failed: usize,
    checks_total: usize,
    checks_success: usize,
    covers_total: usize,
    covers_satisfied: usize,
}

impl Summary {
    fn from_harnesses(harnesses: &[HarnessExport]) -> Self {
        let successful = harnesses.iter().filter(|h| is_successful(&h.outcome)).count();
        let failed = harnesses.iter().filter(|h| is_completed_failure(&h.outcome)).count();
        let checks_total: usize = harnesses.iter().filter_map(|h| h.checks.total).sum();
        let checks_success: usize = harnesses.iter().filter_map(|h| h.checks.success).sum();
        let covers_total: usize = harnesses.iter().filter_map(|h| h.covers.total).sum();
        let covers_satisfied: usize = harnesses
            .iter()
            .filter(|h| h.covers.total.is_some())
            .map(|h| h.covers.satisfied.len())
            .sum();
        Summary {
            total: harnesses.len(),
            successful,
            failed,
            checks_total,
            checks_success,
            covers_total,
            covers_satisfied,
        }
    }
}

impl ExportedRun {
    /// `resolve` returns a harness's `(resolved_solver, resolved_unwind)`.
    fn from_harness_results(
        results: &[HarnessResult<'_>],
        resolve: impl Fn(&HarnessMetadata) -> (Option<String>, Option<u32>),
        ctx: RunContext,
    ) -> Self {
        let mut harnesses: Vec<HarnessExport> = results
            .iter()
            .map(|hr| {
                let (resolved_solver, resolved_unwind) = resolve(hr.harness);
                HarnessExport::from_harness_result(hr, resolved_unwind, resolved_solver)
            })
            .collect();
        harnesses.sort_by(|a, b| {
            (&a.crate_name, &a.file, a.line, &a.name).cmp(&(
                &b.crate_name,
                &b.file,
                b.line,
                &b.name,
            ))
        });
        let mut enabled_unstable_features = ctx.enabled_unstable_features;
        enabled_unstable_features.sort();

        let summary = Summary::from_harnesses(&harnesses);
        let run_state = if ctx.harness_selection.requested_filters.is_empty()
            && ctx.harness_selection.matched_count == 0
        {
            RunState::NoHarnessesSelected
        } else if results.len() == ctx.harness_selection.matched_count {
            RunState::Complete
        } else {
            RunState::Partial
        };

        ExportedRun {
            schema_version: SCHEMA_VERSION,
            kani_commit: ctx.kani_commit,
            kani_commit_dirty: ctx.kani_commit_dirty,
            tools: ToolsExport {
                kani: KANI_VERSION,
                rustc: ctx.rustc_version,
                cbmc: ctx.cbmc_version,
            },
            enabled_unstable_features,
            harness_selection: ctx.harness_selection,
            harness_timeout_s: ctx.harness_timeout_s,
            configuration: ctx.configuration,
            outcome: RunOutcome::Completed,
            run_state,
            target: env!("TARGET"),
            started_at: format_started_at(ctx.started_at),
            wall_time_s: ctx.wall_time.as_secs_f64(),
            harnesses,
            summary,
        }
    }
}

fn format_started_at(dt: OffsetDateTime) -> String {
    let format =
        format_description::parse_borrowed::<2>("[year]-[month]-[day]T[hour]:[minute]:[second]Z")
            .unwrap();
    dt.format(&format).unwrap()
}

impl HarnessExport {
    fn from_harness_result(
        hr: &HarnessResult<'_>,
        resolved_unwind: Option<u32>,
        resolved_solver: Option<String>,
    ) -> Self {
        let harness = hr.harness;
        let result = &hr.result;
        let warnings = result.warnings.clone();
        let warnings_truncated = result.warnings_truncated;

        let (
            outcome,
            n_properties,
            n_failed,
            failure_kind,
            failed_properties,
            unsupported_constructs,
            checks,
            covers,
        ) = match &result.results {
            Ok(properties) => {
                // Excludes code_coverage properties (not exported at all) to hold the
                // exhaustive-partition invariant n_properties == checks.total + covers.total.
                let n_properties =
                    properties.iter().filter(|p| !p.is_code_coverage_property()).count();
                let failed_properties: Vec<PropertyExport> = properties
                    .iter()
                    .filter(|p| !p.is_cover_property() && !p.is_code_coverage_property())
                    .filter(|p| p.status == CheckStatus::Failure)
                    .map(PropertyExport::from_property)
                    .collect();
                let n_failed = failed_properties.len();
                let unsupported_constructs: Vec<PropertyExport> = properties
                    .iter()
                    .filter(|p| p.is_unsupported_construct_property())
                    .map(PropertyExport::from_property)
                    .collect();
                let checks = ChecksExport::from_properties(properties);
                let covers = CoversExport::from_properties(properties);
                let verdict = if result.status == VerificationStatus::Success {
                    Verdict::Success
                } else {
                    Verdict::Failure
                };
                (
                    Outcome::Completed { verdict },
                    Some(n_properties),
                    Some(n_failed),
                    Some(result.failed_properties),
                    failed_properties,
                    unsupported_constructs,
                    checks,
                    covers,
                )
            }
            Err(exit_status) => (
                Outcome::from(exit_status),
                None,
                None,
                None,
                Vec::new(),
                Vec::new(),
                ChecksExport::empty_not_completed(),
                CoversExport::empty_not_completed(),
            ),
        };

        HarnessExport {
            name: harness.pretty_name.clone(),
            crate_name: harness.crate_name.clone(),
            file: relativize_path(&harness.original_file),
            line: harness.original_start_line,
            contract: harness.contract.clone(),
            is_automatically_generated: harness.is_automatically_generated,
            has_loop_contracts: harness.has_loop_contracts,
            is_bounded: harness.is_bounded,
            attributes: harness.attributes.clone(),
            outcome,
            resolved_solver,
            resolved_unwind,
            generated_concrete_test: result.generated_concrete_test,
            resources: ResourcesExport { verification_time_s: result.runtime.as_secs_f64() },
            n_properties,
            n_failed,
            failure_kind,
            failed_properties,
            unsupported_constructs,
            warnings,
            warnings_truncated,
            checks,
            covers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_cbmc::VerificationResult;
    use crate::cbmc_output_parser::{PropertyId, SourceLocation};
    use kani_metadata::{HarnessKind, HarnessMetadata, Stub};

    fn harness(pretty: &str) -> HarnessMetadata {
        harness_in_crate(pretty, "krate")
    }

    fn harness_in_crate(pretty: &str, crate_name: &str) -> HarnessMetadata {
        HarnessMetadata {
            pretty_name: pretty.to_string(),
            mangled_name: "mangled".to_string(),
            crate_name: crate_name.to_string(),
            original_file: "src/lib.rs".to_string(),
            original_start_line: 10,
            original_end_line: 20,
            goto_file: None,
            attributes: HarnessAttributes::new(HarnessKind::Proof),
            contract: None,
            has_loop_contracts: false,
            is_automatically_generated: false,
            is_bounded: false,
            is_ctor_based: false,
        }
    }

    fn harness_at(pretty: &str, crate_name: &str, rel_file: &str, line: usize) -> HarnessMetadata {
        let abs_file =
            std::env::current_dir().unwrap().join(rel_file).to_string_lossy().into_owned();
        HarnessMetadata {
            original_file: abs_file,
            original_start_line: line,
            ..harness_in_crate(pretty, crate_name)
        }
    }

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

    fn success_result(properties: Vec<Property>) -> VerificationResult {
        VerificationResult {
            status: VerificationStatus::Success,
            failed_properties: FailedProperties::None,
            results: Ok(properties),
            runtime: Duration::from_millis(329),
            generated_concrete_test: false,
            ignored_quantifiers: 0,
            coverage_results: None,
            warnings: Vec::new(),
            warnings_truncated: 0,
        }
    }

    fn started() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_754_500_902).unwrap()
    }

    fn test_checks_flags() -> ChecksFlags {
        ChecksFlags {
            memory_safety: true,
            overflow: true,
            unwinding: true,
            undefined_function: true,
            assertion_reach_checks: true,
            ignore_global_asm: false,
            extra_pointer_checks: false,
            assert_contracts: true,
            prove_safety_only: false,
        }
    }

    fn test_ctx_config() -> ConfigurationExport {
        ConfigurationExport {
            checks: test_checks_flags(),
            cbmc_args: Vec::new(),
            coverage_enabled: false,
        }
    }

    fn test_context() -> RunContext {
        RunContext {
            cbmc_version: None,
            rustc_version: None,
            kani_commit: None,
            kani_commit_dirty: None,
            enabled_unstable_features: Vec::new(),
            harness_selection: HarnessSelectionExport {
                requested_filters: Vec::new(),
                exact: false,
                unmatched_filters: Vec::new(),
                matched_count: 1,
            },
            harness_timeout_s: None,
            configuration: test_ctx_config(),
            started_at: started(),
            wall_time: Duration::from_millis(1),
        }
    }

    fn cadical(_: &HarnessMetadata) -> (Option<String>, Option<u32>) {
        (Some("cadical".to_string()), None)
    }

    fn export_with(results: &[HarnessResult<'_>], ctx: RunContext) -> ExportedRun {
        ExportedRun::from_harness_results(results, cadical, ctx)
    }

    fn export_one(hr: HarnessResult<'_>) -> ExportedRun {
        export_with(&[hr], test_context())
    }

    #[test]
    fn export_all_successful() {
        let h = harness("my_harness");
        let properties = vec![
            property("assertion", 1, CheckStatus::Success),
            property("cover", 1, CheckStatus::Satisfied),
        ];
        let result = success_result(properties);
        let hr = HarnessResult { harness: &h, result };

        let ctx = RunContext {
            cbmc_version: Some("CBMC 6.8.0".to_string()),
            wall_time: Duration::from_millis(500),
            ..test_context()
        };
        let export = export_with(&[hr], ctx);
        let v = serde_json::to_value(&export).unwrap();

        assert_eq!(v["schema_version"], "0.1.0");
        assert_eq!(v["outcome"]["kind"], "COMPLETED");
        assert!(v["outcome"].get("verdict").is_none());
        assert_eq!(v["run_state"], "COMPLETE");
        assert_eq!(v["tools"]["cbmc"], "CBMC 6.8.0");
        assert!(v["tools"]["kani"].as_str().is_some_and(|k| !k.is_empty()), "tools.kani: {v}");
        assert!(v.get("solver").is_none(), "top-level solver must be cut entirely");
        assert!(v["summary"].is_object());
        assert_eq!(v["summary"]["total"], 1);
        assert_eq!(v["summary"]["successful"], 1);
        assert_eq!(v["summary"]["failed"], 0);
        assert_eq!(v["summary"]["covers_total"], 1);
        assert_eq!(v["summary"]["covers_satisfied"], 1);
        assert_eq!(v["summary"]["checks_total"], 1);
        assert_eq!(v["summary"]["checks_success"], 1);
        assert_eq!(v["configuration"]["checks"]["memory_safety"], true);
        assert!(v["configuration"]["cbmc_args"].as_array().unwrap().is_empty());
        assert!(v["target"].as_str().is_some_and(|t| !t.is_empty()));
        assert!(v["wall_time_s"].is_number());

        let hj = &v["harnesses"][0];
        assert_eq!(hj["name"], "my_harness");
        assert_eq!(hj["crate_name"], "krate");
        assert_eq!(hj["line"], 10);
        assert_eq!(hj["is_automatically_generated"], false);
        assert_eq!(hj["outcome"]["kind"], "COMPLETED");
        assert_eq!(hj["outcome"]["verdict"], "SUCCESS");
        assert_eq!(hj["resolved_solver"], "cadical");
        assert_eq!(hj["failure_kind"], "NONE");
        assert_eq!(hj["n_properties"], 2);
        assert_eq!(hj["n_failed"], 0);
        assert!(hj["failed_properties"].as_array().unwrap().is_empty());
        assert_eq!(hj["covers"]["total"], 1);
        assert_eq!(
            hj["covers"]["satisfied"].as_array().unwrap(),
            &vec![serde_json::Value::String("harness.cover.1".to_string())]
        );
        assert_eq!(hj["checks"]["total"], 1);
        assert_eq!(hj["checks"]["success"], 1);
        assert_eq!(hj["resources"]["verification_time_s"].as_f64().unwrap(), 0.329);
        assert!(hj["warnings"].as_array().unwrap().is_empty());
        assert_eq!(hj["warnings_truncated"], 0);
    }

    #[test]
    fn export_with_failed_property() {
        let h = harness("my_harness");
        let properties = vec![property("assertion", 1, CheckStatus::Failure)];
        let mut result = success_result(properties);
        result.status = VerificationStatus::Failure;
        result.failed_properties = FailedProperties::PanicsOnly;
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();

        assert!(v["tools"]["cbmc"].is_null());
        let hj = &v["harnesses"][0];
        assert_eq!(hj["outcome"]["verdict"], "FAILURE");
        assert_eq!(hj["failure_kind"], "PANICS_ONLY");
        assert_eq!(hj["n_failed"], 1);
        assert_eq!(hj["failed_properties"][0]["id"], "harness.assertion.1");
        assert_eq!(hj["failed_properties"][0]["description"], "assertion check");
        assert_eq!(hj["failed_properties"][0]["class"], "assertion");
        assert_eq!(hj["failed_properties"][0]["file"], "src/lib.rs");
        assert_eq!(
            hj["checks"]["failure"].as_array().unwrap(),
            &vec![serde_json::Value::String("harness.assertion.1".to_string())]
        );
    }

    #[test]
    fn export_failed_property_full_record_fields() {
        let h = harness("full_record_fields_harness");
        let mut p = property("assertion", 1, CheckStatus::Failure);
        p.source_location.file = None;
        p.trace = Some(Vec::new());
        let mut result = success_result(vec![p]);
        result.status = VerificationStatus::Failure;
        result.failed_properties = FailedProperties::PanicsOnly;
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        let fp = &v["harnesses"][0]["failed_properties"][0];
        assert!(fp["file"].is_null(), "file: {fp}");
        assert_eq!(fp["line"], "12");
        assert_eq!(fp["trace_available"], true);
    }

    #[test]
    fn export_checks_unreachable_under_contradictory_assume() {
        let h = harness("check_contradictory_assume");
        let properties = vec![
            property("assertion", 1, CheckStatus::Unreachable),
            property("assertion", 2, CheckStatus::Unreachable),
        ];
        let result = success_result(properties);
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();

        let hj = &v["harnesses"][0];
        assert_eq!(hj["outcome"]["verdict"], "SUCCESS");
        assert_eq!(hj["n_failed"], 0);
        assert_eq!(hj["covers"]["total"], 0);
        let checks = &hj["checks"];
        assert_eq!(checks["total"], 2);
        assert_eq!(checks["success"], 0);
        let unreachable: Vec<&str> =
            checks["unreachable"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(unreachable, vec!["harness.assertion.1", "harness.assertion.2"]);
    }

    #[test]
    fn export_checks_invariant_holds() {
        let h = harness("mixed_checks_harness");
        let properties = vec![
            property("assertion", 1, CheckStatus::Success),
            property("assertion", 2, CheckStatus::Failure),
            property("assertion", 3, CheckStatus::Unreachable),
            property("assertion", 4, CheckStatus::Undetermined),
            property("assertion", 5, CheckStatus::Error),
            property("assertion", 6, CheckStatus::Unknown),
        ];
        let result = success_result(properties);
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        let checks = &v["harnesses"][0]["checks"];
        let sum = checks["success"].as_u64().unwrap()
            + checks["failure"].as_array().unwrap().len() as u64
            + checks["unreachable"].as_array().unwrap().len() as u64
            + checks["undetermined"].as_array().unwrap().len() as u64
            + checks["error"].as_array().unwrap().len() as u64
            + checks["unknown"].as_array().unwrap().len() as u64
            + checks["other"].as_array().unwrap().len() as u64;
        assert_eq!(sum, checks["total"].as_u64().unwrap());
        assert_eq!(checks["total"], 6);
        assert_eq!(checks["unreachable"].as_array().unwrap().len(), 1);
        assert_eq!(checks["undetermined"].as_array().unwrap().len(), 1);
        assert_eq!(checks["error"].as_array().unwrap().len(), 1);
        assert_eq!(checks["unknown"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn export_checks_satisfied_status_falls_to_other() {
        let h = harness("checks_satisfied_status_harness");
        let properties = vec![property("assertion", 1, CheckStatus::Satisfied)];
        let result = success_result(properties);
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        let checks = &v["harnesses"][0]["checks"];
        assert_eq!(checks["success"], 0);
        assert_eq!(checks["other"].as_array().unwrap().len(), 1);
        assert_eq!(checks["other"][0]["id"], "harness.assertion.1");
        assert_eq!(checks["other"][0]["status"], "SATISFIED");
    }

    #[test]
    fn export_checks_excludes_covers_and_code_coverage() {
        let h = harness("mixed_property_kinds_harness");
        let properties = vec![
            property("assertion", 1, CheckStatus::Success),
            property("cover", 1, CheckStatus::Satisfied),
            property("code_coverage", 1, CheckStatus::Covered),
        ];
        let result = success_result(properties);
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        assert_eq!(v["harnesses"][0]["checks"]["total"], 1);
        assert_eq!(v["harnesses"][0]["covers"]["total"], 1);
    }

    #[test]
    fn export_n_properties_excludes_code_coverage_under_coverage() {
        let h = harness("coverage_harness");
        let properties = vec![
            property("assertion", 1, CheckStatus::Success),
            property("cover", 1, CheckStatus::Satisfied),
            property("code_coverage", 1, CheckStatus::Covered),
            property("code_coverage", 2, CheckStatus::Uncovered),
        ];
        let result = success_result(properties);
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        let hj = &v["harnesses"][0];
        let checks_total = hj["checks"]["total"].as_u64().unwrap();
        let covers_total = hj["covers"]["total"].as_u64().unwrap();
        assert_eq!(hj["n_properties"], 2, "excludes the two code_coverage properties");
        assert_eq!(hj["n_properties"].as_u64().unwrap(), checks_total + covers_total);
    }

    fn assert_harness_shape_fields_present(hj: &serde_json::Value) {
        assert!(hj["name"].as_str().is_some_and(|s| !s.is_empty()), "name: {hj}");
        assert!(hj["crate_name"].as_str().is_some_and(|s| !s.is_empty()), "crate_name: {hj}");
        assert!(hj["file"].is_string(), "file: {hj}");
        assert!(hj["line"].is_number(), "line: {hj}");
        assert!(hj.get("contract").is_some(), "contract: {hj}");
        assert!(hj["is_automatically_generated"].is_boolean(), "is_automatically_generated: {hj}");
        assert!(hj["has_loop_contracts"].is_boolean(), "has_loop_contracts: {hj}");
        assert!(hj["is_bounded"].is_boolean(), "is_bounded: {hj}");
        assert!(hj["attributes"].is_object(), "attributes: {hj}");
        assert!(hj.get("resolved_solver").is_some(), "resolved_solver: {hj}");
        assert!(hj.get("resolved_unwind").is_some(), "resolved_unwind: {hj}");
        assert!(hj["generated_concrete_test"].is_boolean(), "generated_concrete_test: {hj}");
    }

    fn assert_not_completed_matrix_fields(hj: &serde_json::Value) {
        assert!(hj["checks"].is_object(), "checks should still be an object: {hj}");
        assert!(hj["covers"].is_object(), "covers should still be an object: {hj}");
        for field in ["n_properties", "n_failed"] {
            assert_eq!(
                hj.get(field),
                Some(&serde_json::Value::Null),
                "{field} should be present and null: {hj}"
            );
        }
        assert!(hj.get("failure_kind").is_none(), "failure_kind should be absent: {hj}");
        for field in ["total", "success"] {
            assert_eq!(
                hj["checks"].get(field),
                Some(&serde_json::Value::Null),
                "checks.{field} should be present and null: {hj}"
            );
        }
        assert_eq!(
            hj["covers"].get("total"),
            Some(&serde_json::Value::Null),
            "covers.total should be present and null: {hj}"
        );
        for field in ["failure", "unreachable", "undetermined", "error", "unknown", "other"] {
            assert_eq!(
                hj["checks"].get(field),
                Some(&serde_json::json!([])),
                "checks.{field} should be present and empty: {hj}"
            );
        }
        for field in [
            "satisfied",
            "unsatisfiable",
            "unreachable",
            "undetermined",
            "error",
            "unknown",
            "other",
        ] {
            assert_eq!(
                hj["covers"].get(field),
                Some(&serde_json::json!([])),
                "covers.{field} should be present and empty: {hj}"
            );
        }
        assert_eq!(hj.get("failed_properties"), Some(&serde_json::json!([])));
        assert_eq!(hj.get("unsupported_constructs"), Some(&serde_json::json!([])));
    }

    #[test]
    fn export_with_exit_status_outcomes() {
        let h = harness("crashed_harness");
        let mk = |exit: ExitStatus| VerificationResult {
            status: VerificationStatus::Failure,
            failed_properties: FailedProperties::None,
            results: Err(exit),
            runtime: Duration::from_secs(1),
            generated_concrete_test: false,
            ignored_quantifiers: 0,
            coverage_results: None,
            warnings: Vec::new(),
            warnings_truncated: 0,
        };

        let timeout_v = serde_json::to_value(export_one(HarnessResult {
            harness: &h,
            result: mk(ExitStatus::Timeout),
        }))
        .unwrap();
        assert_eq!(timeout_v["harnesses"][0]["outcome"]["kind"], "TIMEOUT");
        assert!(timeout_v["harnesses"][0]["outcome"].get("verdict").is_none());
        assert!(timeout_v["harnesses"][0]["outcome"].get("code").is_none());
        assert!(timeout_v["harnesses"][0]["outcome"].get("message").is_none());
        assert_eq!(timeout_v["harnesses"][0]["resources"]["verification_time_s"], 1.0);
        assert_harness_shape_fields_present(&timeout_v["harnesses"][0]);
        assert_not_completed_matrix_fields(&timeout_v["harnesses"][0]);

        let oom_v = serde_json::to_value(export_one(HarnessResult {
            harness: &h,
            result: mk(ExitStatus::OutOfMemory),
        }))
        .unwrap();
        assert_eq!(oom_v["harnesses"][0]["outcome"]["kind"], "OUT_OF_MEMORY");
        assert!(oom_v["harnesses"][0]["outcome"].get("verdict").is_none());
        assert!(oom_v["harnesses"][0]["outcome"].get("code").is_none());
        assert!(oom_v["harnesses"][0]["outcome"].get("message").is_none());
        assert_eq!(oom_v["harnesses"][0]["resources"]["verification_time_s"], 1.0);
        assert_harness_shape_fields_present(&oom_v["harnesses"][0]);
        assert_not_completed_matrix_fields(&oom_v["harnesses"][0]);

        let crashed_v = serde_json::to_value(export_one(HarnessResult {
            harness: &h,
            result: mk(ExitStatus::Other(101)),
        }))
        .unwrap();
        assert_eq!(crashed_v["harnesses"][0]["outcome"]["kind"], "CRASHED");
        assert_eq!(crashed_v["harnesses"][0]["outcome"]["code"], 101);
        assert!(crashed_v["harnesses"][0]["outcome"].get("verdict").is_none());
        assert_eq!(crashed_v["harnesses"][0]["outcome"]["message"], "CBMC failed with status 101");
        assert_eq!(crashed_v["harnesses"][0]["resources"]["verification_time_s"], 1.0);
        assert_harness_shape_fields_present(&crashed_v["harnesses"][0]);
        assert_not_completed_matrix_fields(&crashed_v["harnesses"][0]);
    }

    /// The exported outcome for a CBMC run with one reported property, by exit status and by
    /// whether CBMC reported `Out of memory` after the result array.
    #[test]
    fn export_outcome_follows_cbmc_exit_status_and_results() {
        use crate::cbmc_output_parser::{ParserItem, VerificationOutput};
        use std::time::Instant;

        // (status, property status, `Out of memory` reported, kind, code, failure_kind)
        let cases = [
            (0, CheckStatus::Success, false, "COMPLETED", None, Some("NONE")),
            (10, CheckStatus::Failure, false, "COMPLETED", None, Some("PANICS_ONLY")),
            (6, CheckStatus::Error, false, "COMPLETED", None, Some("ERROR")),
            (6, CheckStatus::Success, true, "OUT_OF_MEMORY", None, None),
            (6, CheckStatus::Error, true, "OUT_OF_MEMORY", None, None),
            (137, CheckStatus::Success, false, "OUT_OF_MEMORY", None, None),
            (6, CheckStatus::Success, false, "CRASHED", Some(6), None),
            (5, CheckStatus::Unknown, false, "CRASHED", Some(5), None),
            (139, CheckStatus::Success, false, "CRASHED", Some(139), None),
        ];
        for (status, property_status, reported_oom, kind, code, failure_kind) in cases {
            let mut processed_items = vec![ParserItem::Result {
                result: vec![property("assertion", 1, property_status)],
            }];
            if reported_oom {
                processed_items.push(ParserItem::Message {
                    message_text: "Out of memory".to_string(),
                    message_type: "ERROR".to_string(),
                });
            }
            let output = VerificationOutput { process_status: status, processed_items };
            let result = VerificationResult::from(output, false, Instant::now());
            let h = harness("h");
            let v =
                serde_json::to_value(export_one(HarnessResult { harness: &h, result })).unwrap();
            let hj = &v["harnesses"][0];
            let case = format!("status {status}, {property_status:?}, oom {reported_oom}: {hj}");
            assert_eq!(hj["outcome"]["kind"], kind, "{case}");
            assert_eq!(hj["outcome"].get("code").and_then(|c| c.as_i64()), code, "{case}");
            assert_eq!(hj.get("failure_kind").and_then(|f| f.as_str()), failure_kind, "{case}");
            if kind == "COMPLETED" {
                assert_eq!(hj["n_properties"], 1, "{case}");
            } else {
                assert_not_completed_matrix_fields(hj);
            }
            if kind == "CRASHED" {
                assert_eq!(
                    hj["outcome"]["message"],
                    format!("CBMC failed with status {status}"),
                    "{case}"
                );
            }
        }
    }

    #[test]
    fn export_error_property_excluded_from_failed_properties_but_sets_failure_kind() {
        let h = harness("solver_error_harness");
        let properties = vec![
            property("assertion", 1, CheckStatus::Success),
            property("assertion", 2, CheckStatus::Error),
            property("unsupported_construct", 3, CheckStatus::Error),
        ];
        let mut result = success_result(properties);
        result.status = VerificationStatus::Failure;
        result.failed_properties = FailedProperties::Error;
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        let hj = &v["harnesses"][0];
        assert_eq!(hj["outcome"]["verdict"], "FAILURE");
        assert_eq!(hj["failure_kind"], "ERROR");
        assert_eq!(hj["n_failed"], 0);
        assert!(hj["failed_properties"].as_array().unwrap().is_empty());
        let checks_error: Vec<&str> =
            hj["checks"]["error"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
        assert_eq!(checks_error, vec!["harness.assertion.2", "harness.unsupported_construct.3"]);
        assert_eq!(hj["unsupported_constructs"][0]["id"], "harness.unsupported_construct.3");
        assert_eq!(hj["unsupported_constructs"][0]["status"], "ERROR");
    }

    #[test]
    fn export_failed_properties_matches_checks_failure_exactly() {
        let h = harness("class_and_status_filtered_failures_harness");
        let properties = vec![
            property("assertion", 1, CheckStatus::Failure),
            property("cover", 2, CheckStatus::Failure),
            property("code_coverage", 3, CheckStatus::Failure),
            property("assertion", 4, CheckStatus::Error),
        ];
        let mut result = success_result(properties);
        result.status = VerificationStatus::Failure;
        result.failed_properties = FailedProperties::PanicsOnly;
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        let hj = &v["harnesses"][0];
        let failed_ids: Vec<&str> = hj["failed_properties"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_str().unwrap())
            .collect();
        assert_eq!(failed_ids, vec!["harness.assertion.1"]);
        assert_eq!(hj["n_failed"], 1);
        let checks_failure: Vec<&str> = hj["checks"]["failure"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        assert_eq!(
            failed_ids, checks_failure,
            "failed_properties[] must be exactly checks.failure"
        );
    }

    fn sum_cover_buckets(covers: &serde_json::Value) -> u64 {
        covers["satisfied"].as_array().unwrap().len() as u64
            + covers["unsatisfiable"].as_array().unwrap().len() as u64
            + covers["unreachable"].as_array().unwrap().len() as u64
            + covers["undetermined"].as_array().unwrap().len() as u64
            + covers["error"].as_array().unwrap().len() as u64
            + covers["unknown"].as_array().unwrap().len() as u64
            + covers["other"].as_array().unwrap().len() as u64
    }

    #[test]
    fn export_covers_all_four_states_and_invariant() {
        let h = harness("mixed_covers_harness");
        let properties = vec![
            property("cover", 1, CheckStatus::Satisfied),
            property("cover", 2, CheckStatus::Unsatisfiable),
            property("cover", 3, CheckStatus::Unreachable),
            property("cover", 4, CheckStatus::Undetermined),
        ];
        let result = success_result(properties);
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        let covers = &v["harnesses"][0]["covers"];
        assert_eq!(covers["total"], 4);
        assert_eq!(
            covers["satisfied"].as_array().unwrap(),
            &vec![serde_json::Value::String("harness.cover.1".to_string())]
        );
        assert_eq!(
            covers["unsatisfiable"].as_array().unwrap(),
            &vec![serde_json::Value::String("harness.cover.2".to_string())]
        );
        assert_eq!(sum_cover_buckets(covers), covers["total"].as_u64().unwrap());
    }

    #[test]
    fn export_covers_unexpected_status_goes_to_other() {
        let h = harness("unexpected_cover_status_harness");
        let properties = vec![property("cover", 1, CheckStatus::Success)];
        let result = success_result(properties);
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        let covers = &v["harnesses"][0]["covers"];
        assert_eq!(covers["other"].as_array().unwrap().len(), 1);
        assert_eq!(covers["other"][0]["id"], "harness.cover.1");
        assert_eq!(covers["other"][0]["status"], "SUCCESS");
        assert_eq!(sum_cover_buckets(covers), covers["total"].as_u64().unwrap());
    }

    #[test]
    fn export_unsupported_construct_listed_separately() {
        let h = harness("volatile_probe_harness");
        let properties = vec![
            property("assertion", 1, CheckStatus::Failure),
            property("unsupported_construct", 1, CheckStatus::Failure),
        ];
        let mut result = success_result(properties);
        result.status = VerificationStatus::Failure;
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        let hj = &v["harnesses"][0];
        assert_eq!(hj["n_failed"], 2);
        let unsupported = hj["unsupported_constructs"].as_array().unwrap();
        assert_eq!(unsupported.len(), 1);
        assert_eq!(unsupported[0]["id"], "harness.unsupported_construct.1");
        assert_eq!(unsupported[0]["class"], "unsupported_construct");
        assert!(!unsupported.iter().any(|p| p["id"] == "harness.assertion.1"));
        let failed = hj["failed_properties"].as_array().unwrap();
        assert!(failed.iter().any(|p| p["id"] == "harness.unsupported_construct.1"));
    }

    #[test]
    fn export_kani_commit_and_dirty_flag() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let ctx = RunContext {
            kani_commit: Some("d4df833c8f8f18e632e7b0a7945bb2161f708990"),
            kani_commit_dirty: Some(true),
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&[hr], ctx)).unwrap();
        assert_eq!(v["kani_commit"], "d4df833c8f8f18e632e7b0a7945bb2161f708990");
        assert_eq!(v["kani_commit_dirty"], true);
    }

    #[test]
    fn export_kani_commit_null_when_unavailable() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v = serde_json::to_value(export_one(hr)).unwrap();
        assert!(v["kani_commit"].is_null());
        assert!(v["kani_commit_dirty"].is_null());
    }

    #[test]
    fn export_tools_rustc_null_when_unavailable() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v = serde_json::to_value(export_one(hr)).unwrap();
        assert!(v["tools"]["rustc"].is_null());
    }

    #[test]
    fn export_tools_rustc_present_when_available() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let ctx = RunContext {
            rustc_version: Some("1.98.0-nightly (14210df0e 2026-05-31)".to_string()),
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&[hr], ctx)).unwrap();
        assert_eq!(v["tools"]["rustc"], "1.98.0-nightly (14210df0e 2026-05-31)");
    }

    /// `tools.rustc` is the toolchain release, not the `using rustc ... with LLVM ...` sentence
    /// `kani --version --verbose` prints.
    #[test]
    fn exported_rustc_is_the_build_time_toolchain_release() {
        let release = build_rustc_release().expect("build.rs records the rustc release");
        let shape = regex::Regex::new(r"^\d+\.\d+\.\d+\S* \(").unwrap();
        assert!(shape.is_match(&release), "unexpected tools.rustc value: {release:?}");
        assert!(!release.starts_with("using") && !release.starts_with("rustc "), "{release:?}");
    }

    #[test]
    fn export_enabled_unstable_features_sorted() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let ctx = RunContext {
            enabled_unstable_features: vec![
                "quantifiers".to_string(),
                "function-contracts".to_string(),
            ],
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&[hr], ctx)).unwrap();
        assert_eq!(
            v["enabled_unstable_features"].as_array().unwrap(),
            &vec![
                serde_json::Value::String("function-contracts".to_string()),
                serde_json::Value::String("quantifiers".to_string())
            ]
        );
    }

    #[test]
    fn export_harness_selection_and_configuration() {
        let h = harness("check_volatile_load_wrapper_contract");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let ctx = RunContext {
            harness_selection: HarnessSelectionExport {
                requested_filters: vec!["check_volatile_load_wrapper_contract".to_string()],
                exact: true,
                unmatched_filters: Vec::new(),
                matched_count: 1,
            },
            configuration: ConfigurationExport {
                checks: ChecksFlags {
                    memory_safety: false,
                    overflow: false,
                    unwinding: false,
                    undefined_function: false,
                    assertion_reach_checks: false,
                    ignore_global_asm: true,
                    extra_pointer_checks: true,
                    assert_contracts: true,
                    prove_safety_only: false,
                },
                cbmc_args: vec!["--object-bits".to_string(), "16".to_string()],
                coverage_enabled: false,
            },
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&[hr], ctx)).unwrap();
        assert_eq!(
            v["harness_selection"]["requested_filters"],
            serde_json::json!(["check_volatile_load_wrapper_contract"])
        );
        assert_eq!(v["harness_selection"]["exact"], true);
        assert_eq!(v["harness_selection"]["matched_count"], 1);
        assert_eq!(v["configuration"]["checks"]["memory_safety"], false);
        assert_eq!(v["configuration"]["checks"]["overflow"], false);
        assert_eq!(v["configuration"]["checks"]["unwinding"], false);
        assert_eq!(v["configuration"]["checks"]["undefined_function"], false);
        assert_eq!(v["configuration"]["checks"]["assertion_reach_checks"], false);
        assert_eq!(v["configuration"]["checks"]["ignore_global_asm"], true);
        assert_eq!(v["configuration"]["checks"]["extra_pointer_checks"], true);
        assert_eq!(v["configuration"]["checks"]["assert_contracts"], true);
        assert_eq!(v["configuration"]["checks"]["prove_safety_only"], false);
        assert_eq!(
            v["configuration"]["cbmc_args"].as_array().unwrap(),
            &vec![
                serde_json::Value::String("--object-bits".to_string()),
                serde_json::Value::String("16".to_string())
            ]
        );
    }

    #[test]
    fn export_configuration_coverage_enabled() {
        let h = harness("h");

        let v_off = serde_json::to_value(export_one(HarnessResult {
            harness: &h,
            result: success_result(vec![]),
        }))
        .unwrap();
        assert_eq!(v_off["configuration"]["coverage_enabled"], false);

        let ctx = RunContext {
            configuration: ConfigurationExport { coverage_enabled: true, ..test_ctx_config() },
            ..test_context()
        };
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v_on = serde_json::to_value(export_with(&[hr], ctx)).unwrap();
        assert_eq!(v_on["configuration"]["coverage_enabled"], true);
    }

    #[test]
    fn export_configuration_assert_contracts() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let ctx = RunContext {
            configuration: ConfigurationExport {
                checks: ChecksFlags { assert_contracts: false, ..test_checks_flags() },
                ..test_ctx_config()
            },
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&[hr], ctx)).unwrap();
        assert_eq!(v["configuration"]["checks"]["assert_contracts"], false);
    }

    #[test]
    fn export_configuration_prove_safety_only() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let ctx = RunContext {
            configuration: ConfigurationExport {
                checks: ChecksFlags { prove_safety_only: true, ..test_checks_flags() },
                ..test_ctx_config()
            },
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&[hr], ctx)).unwrap();
        assert_eq!(v["configuration"]["checks"]["prove_safety_only"], true);
    }

    #[test]
    fn compute_unmatched_filters_flags_the_nonmatching_one() {
        let h = harness("check_volatile_load_wrapper_contract");
        let matched: Vec<&HarnessMetadata> = vec![&h];
        let requested = vec![
            "check_volatile_load_wrapper_contract".to_string(),
            "check_totally_bogus_typo".to_string(),
        ];
        let unmatched = compute_unmatched_filters(&requested, &matched, false);
        assert_eq!(unmatched, vec!["check_totally_bogus_typo".to_string()]);
    }

    #[test]
    fn compute_unmatched_filters_respects_substring_matching() {
        let h = harness("mymod::check_volatile_load_wrapper_contract");
        let matched: Vec<&HarnessMetadata> = vec![&h];
        let requested = vec!["volatile_load".to_string()];
        assert!(compute_unmatched_filters(&requested, &matched, false).is_empty());
        assert_eq!(
            compute_unmatched_filters(&requested, &matched, true),
            vec!["volatile_load".to_string()]
        );
    }

    #[test]
    fn export_run_level_outcome_has_no_crashed_variant() {
        let ctx = RunContext {
            harness_selection: HarnessSelectionExport {
                requested_filters: Vec::new(),
                exact: false,
                unmatched_filters: Vec::new(),
                matched_count: 0,
            },
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&[], ctx)).unwrap();
        assert_eq!(v["outcome"], serde_json::json!({ "kind": "COMPLETED" }));
        assert_eq!(v["run_state"], "NO_HARNESSES_SELECTED");
        assert!(v["harnesses"].as_array().unwrap().is_empty());
    }

    #[test]
    fn export_run_state_partial_when_fewer_results_than_matched() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let ctx = RunContext {
            harness_selection: HarnessSelectionExport {
                requested_filters: Vec::new(),
                exact: false,
                unmatched_filters: Vec::new(),
                matched_count: 50,
            },
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&[hr], ctx)).unwrap();
        assert_eq!(v["outcome"]["kind"], "COMPLETED");
        assert_eq!(v["run_state"], "PARTIAL");
        assert_eq!(v["summary"]["total"], 1);
    }

    #[test]
    fn export_summary_fields_computed_over_completed_only() {
        let h1 = harness_in_crate("completed_harness", "krate");
        let properties = vec![
            property("assertion", 1, CheckStatus::Success),
            property("cover", 1, CheckStatus::Satisfied),
        ];
        let hr1 = HarnessResult { harness: &h1, result: success_result(properties) };
        let completed = HarnessExport::from_harness_result(&hr1, None, Some("cadical".to_string()));

        let h2 = harness_in_crate("timeout_harness", "krate");
        let contrived_timeout = HarnessExport {
            name: h2.pretty_name.clone(),
            crate_name: h2.crate_name.clone(),
            file: "src/lib.rs".to_string(),
            line: 10,
            contract: None,
            is_automatically_generated: false,
            has_loop_contracts: false,
            is_bounded: false,
            attributes: h2.attributes.clone(),
            outcome: Outcome::Timeout,
            resolved_solver: None,
            resolved_unwind: None,
            generated_concrete_test: false,
            resources: ResourcesExport { verification_time_s: 1.0 },
            n_properties: None,
            n_failed: None,
            failure_kind: None,
            failed_properties: Vec::new(),
            unsupported_constructs: Vec::new(),
            warnings: Vec::new(),
            warnings_truncated: 0,
            checks: ChecksExport::empty_not_completed(),
            covers: CoversExport {
                satisfied: vec!["bogus.cover.1".to_string()],
                ..CoversExport::empty_not_completed()
            },
        };

        let summary = Summary::from_harnesses(&[completed, contrived_timeout]);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.successful, 1);
        assert_eq!(
            summary.failed, 0,
            "a non-COMPLETED harness must count in neither successful nor failed"
        );
        assert_eq!(summary.checks_total, 1);
        assert_eq!(summary.checks_success, 1);
        assert_eq!(summary.covers_total, 1);
        assert_eq!(
            summary.covers_satisfied, 1,
            "the contrived satisfied entry off-COMPLETED (covers.total == None) must not leak in"
        );
    }

    #[test]
    fn export_harnesses_sorted_by_crate_file_line_name() {
        let h_z_in_a = harness_in_crate("z_harness", "crate_a");
        let h_a_in_b = harness_in_crate("a_harness", "crate_b");
        let h_a_in_a = harness_in_crate("a_harness", "crate_a");
        let h_in_z_file = harness_at("a_name_in_z_file", "crate_a", "z_file.rs", 1);
        let h_later_line = harness_at("a_name_later_line", "crate_a", "src/lib.rs", 999);

        let hr1 = HarnessResult { harness: &h_z_in_a, result: success_result(vec![]) };
        let hr2 = HarnessResult { harness: &h_a_in_b, result: success_result(vec![]) };
        let hr3 = HarnessResult { harness: &h_a_in_a, result: success_result(vec![]) };
        let hr4 = HarnessResult { harness: &h_in_z_file, result: success_result(vec![]) };
        let hr5 = HarnessResult { harness: &h_later_line, result: success_result(vec![]) };

        let ctx = RunContext {
            harness_selection: HarnessSelectionExport {
                requested_filters: Vec::new(),
                exact: false,
                unmatched_filters: Vec::new(),
                matched_count: 5,
            },
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&[hr1, hr2, hr3, hr4, hr5], ctx)).unwrap();
        let keys: Vec<(&str, &str, i64, &str)> = v["harnesses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| {
                (
                    h["crate_name"].as_str().unwrap(),
                    h["file"].as_str().unwrap(),
                    h["line"].as_i64().unwrap(),
                    h["name"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            keys,
            vec![
                ("crate_a", "src/lib.rs", 10, "a_harness"),
                ("crate_a", "src/lib.rs", 10, "z_harness"),
                ("crate_a", "src/lib.rs", 999, "a_name_later_line"),
                ("crate_a", "z_file.rs", 1, "a_name_in_z_file"),
                ("crate_b", "src/lib.rs", 10, "a_harness"),
            ]
        );
    }

    #[test]
    fn export_resolved_unwind_and_has_loop_contracts_and_generated_test() {
        let mut h = harness("h");
        h.has_loop_contracts = true;
        let mut result = success_result(vec![]);
        result.generated_concrete_test = true;
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(ExportedRun::from_harness_results(
            &[hr],
            |_| (Some("cadical".to_string()), Some(7)),
            test_context(),
        ))
        .unwrap();
        let hj = &v["harnesses"][0];
        assert_eq!(hj["resolved_unwind"], 7);
        assert_eq!(hj["has_loop_contracts"], true);
        assert_eq!(hj["generated_concrete_test"], true);
    }

    #[test]
    fn export_resolved_unwind_null_when_unset() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v = serde_json::to_value(export_one(hr)).unwrap();
        assert!(v["harnesses"][0]["resolved_unwind"].is_null());
    }

    #[test]
    fn export_harness_timeout_s() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let ctx = RunContext { harness_timeout_s: Some(30.0), ..test_context() };
        let v = serde_json::to_value(export_with(&[hr], ctx)).unwrap();
        assert_eq!(v["harness_timeout_s"], 30.0);
    }

    #[test]
    fn export_harness_timeout_s_null_when_unset() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v = serde_json::to_value(export_one(hr)).unwrap();
        assert!(v["harness_timeout_s"].is_null());
    }

    #[test]
    fn export_started_at_is_utc_iso8601() {
        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v = serde_json::to_value(export_one(hr)).unwrap();
        let started_at = v["started_at"].as_str().unwrap();
        let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$").unwrap();
        assert!(re.is_match(started_at), "started_at {started_at:?} is not YYYY-MM-DDTHH:MM:SSZ");
    }

    #[test]
    fn export_harness_attributes_round_trip() {
        let mut h = harness("panicking_stubbed_harness");
        h.attributes.should_panic = true;
        h.attributes.unwind_value = Some(5);
        h.attributes.stubs =
            vec![Stub { original: "real_fn".to_string(), replacement: "stub_fn".to_string() }];
        h.attributes.verified_stubs = vec!["verified_target_fn".to_string()];

        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v = serde_json::to_value(export_one(hr)).unwrap();
        let attrs = &v["harnesses"][0]["attributes"];

        assert_eq!(attrs["should_panic"], true);
        assert_eq!(attrs["unwind_value"], 5);
        assert_eq!(
            attrs["stubs"],
            serde_json::json!([{"original": "real_fn", "replacement": "stub_fn"}])
        );
        assert_eq!(attrs["verified_stubs"], serde_json::json!(["verified_target_fn"]));
    }

    #[test]
    fn export_attributes_kind_domain_closed() {
        let mut h_proof = harness("proof_harness");
        h_proof.attributes.kind = HarnessKind::Proof;
        let v_proof = serde_json::to_value(export_one(HarnessResult {
            harness: &h_proof,
            result: success_result(vec![]),
        }))
        .unwrap();
        assert_eq!(v_proof["harnesses"][0]["attributes"]["kind"], "Proof");

        let mut h_test = harness("test_harness");
        h_test.attributes.kind = HarnessKind::Test;
        let v_test = serde_json::to_value(export_one(HarnessResult {
            harness: &h_test,
            result: success_result(vec![]),
        }))
        .unwrap();
        assert_eq!(v_test["harnesses"][0]["attributes"]["kind"], "Test");

        let mut h_pfc = harness("contract_harness");
        h_pfc.attributes.kind = HarnessKind::ProofForContract { target_fn: "f".to_string() };
        let v_pfc = serde_json::to_value(export_one(HarnessResult {
            harness: &h_pfc,
            result: success_result(vec![]),
        }))
        .unwrap();
        assert_eq!(
            v_pfc["harnesses"][0]["attributes"]["kind"],
            serde_json::json!({ "ProofForContract": { "target_fn": "f" } })
        );
    }

    #[test]
    fn export_attributes_solver_serializes_named_variants() {
        let mut h_none = harness("solver_unset_harness");
        h_none.attributes.solver = None;
        let v_none = serde_json::to_value(export_one(HarnessResult {
            harness: &h_none,
            result: success_result(vec![]),
        }))
        .unwrap();
        assert!(v_none["harnesses"][0]["attributes"]["solver"].is_null());

        let mut h_named = harness("solver_named_harness");
        h_named.attributes.solver = Some(CbmcSolver::Cadical);
        let v_named = serde_json::to_value(export_one(HarnessResult {
            harness: &h_named,
            result: success_result(vec![]),
        }))
        .unwrap();
        assert_eq!(v_named["harnesses"][0]["attributes"]["solver"], "Cadical");

        let mut h_binary = harness("solver_binary_harness");
        h_binary.attributes.solver = Some(CbmcSolver::Binary("/opt/solvers/my-solver".to_string()));
        let v_binary = serde_json::to_value(export_one(HarnessResult {
            harness: &h_binary,
            result: success_result(vec![]),
        }))
        .unwrap();
        assert_eq!(
            v_binary["harnesses"][0]["attributes"]["solver"],
            serde_json::json!({ "Binary": "/opt/solvers/my-solver" })
        );
    }

    #[test]
    fn export_solver_domain_accepts_unfamiliar_names() {
        let h = harness("cbmc_args_solver_harness");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v = serde_json::to_value(ExportedRun::from_harness_results(
            &[hr],
            |_| (Some("totally-unfamiliar-solver-v9".to_string()), None),
            test_context(),
        ))
        .unwrap();
        assert_eq!(v["harnesses"][0]["resolved_solver"], "totally-unfamiliar-solver-v9");
    }

    mod configuration_mapping_tests {
        use super::*;
        use crate::args::{StandaloneArgs, ValidateArgs};
        use clap::Parser;

        fn opts(args: &str) -> VerificationArgs {
            let verify_opts =
                StandaloneArgs::try_parse_from(format!("kani input.rs {args}").split_whitespace())
                    .unwrap()
                    .verify_opts;
            verify_opts.validate().unwrap();
            verify_opts
        }

        #[test]
        fn configuration_from_serializes_defaults() {
            let v = serde_json::to_value(configuration_from(&opts(""))).unwrap();
            assert_eq!(v["checks"]["memory_safety"], true);
            assert_eq!(v["checks"]["overflow"], true);
            assert_eq!(v["checks"]["unwinding"], true);
            assert_eq!(v["checks"]["undefined_function"], true);
            assert_eq!(v["checks"]["assertion_reach_checks"], true);
            assert_eq!(v["checks"]["ignore_global_asm"], false);
            assert_eq!(v["checks"]["extra_pointer_checks"], false);
            assert_eq!(v["checks"]["assert_contracts"], true);
            assert_eq!(v["checks"]["prove_safety_only"], false);
            assert_eq!(v["cbmc_args"], serde_json::json!([]));
            assert_eq!(v["coverage_enabled"], false);
        }

        #[test]
        fn configuration_from_serializes_assert_contracts_disabled() {
            let v = serde_json::to_value(configuration_from(&opts(
                "-Z function-contracts --no-assert-contracts",
            )))
            .unwrap();
            assert_eq!(v["checks"]["assert_contracts"], false);
        }

        #[test]
        fn configuration_from_serializes_prove_safety_only_enabled() {
            let v = serde_json::to_value(configuration_from(&opts(
                "-Z unstable-options --prove-safety-only",
            )))
            .unwrap();
            assert_eq!(v["checks"]["prove_safety_only"], true);
        }

        #[test]
        fn configuration_from_serializes_each_check_toggle() {
            let cases: &[(&str, &str)] = &[
                ("--no-memory-safety-checks", "memory_safety"),
                ("--no-overflow-checks", "overflow"),
                ("--no-unwinding-checks", "unwinding"),
                ("--no-undefined-function-checks", "undefined_function"),
            ];
            for (flag, key) in cases {
                let v = serde_json::to_value(configuration_from(&opts(flag))).unwrap();
                assert_eq!(v["checks"][key], false, "{flag} should flip checks.{key} false");
                for (_, other_key) in cases.iter().filter(|(_, k)| k != key) {
                    assert_eq!(
                        v["checks"][other_key], true,
                        "{flag} should not affect checks.{other_key}"
                    );
                }
            }

            let v = serde_json::to_value(configuration_from(&opts("--no-assertion-reach-checks")))
                .unwrap();
            assert_eq!(v["checks"]["assertion_reach_checks"], false);

            let v = serde_json::to_value(configuration_from(&opts(
                "-Z unstable-options --ignore-global-asm",
            )))
            .unwrap();
            assert_eq!(v["checks"]["ignore_global_asm"], true);

            let v = serde_json::to_value(configuration_from(&opts(
                "-Z unstable-options --extra-pointer-checks",
            )))
            .unwrap();
            assert_eq!(v["checks"]["extra_pointer_checks"], true);
        }

        #[test]
        fn configuration_from_serializes_no_default_checks_group() {
            let v = serde_json::to_value(configuration_from(&opts("--no-default-checks"))).unwrap();
            assert_eq!(v["checks"]["memory_safety"], false);
            assert_eq!(v["checks"]["overflow"], false);
            assert_eq!(v["checks"]["unwinding"], false);
            assert_eq!(v["checks"]["undefined_function"], false);
            assert_eq!(v["checks"]["assertion_reach_checks"], true);
            assert_eq!(v["checks"]["ignore_global_asm"], false);
            assert_eq!(v["checks"]["extra_pointer_checks"], false);
        }

        #[test]
        fn configuration_from_serializes_coverage_enabled() {
            let v = serde_json::to_value(configuration_from(&opts(""))).unwrap();
            assert_eq!(v["coverage_enabled"], false);
            let v =
                serde_json::to_value(configuration_from(&opts("-Z source-coverage --coverage")))
                    .unwrap();
            assert_eq!(v["coverage_enabled"], true);
        }

        #[test]
        fn configuration_from_wires_cbmc_args_verbatim() {
            let config = configuration_from(&opts(
                "-Zunstable-options --cbmc-args --zzz-flag --object-bits 16",
            ));
            assert_eq!(
                config.cbmc_args,
                vec!["--zzz-flag".to_string(), "--object-bits".to_string(), "16".to_string()]
            );
        }
    }

    /// Every case is checked against CBMC 6.11.0's own behavior.
    mod effective_solver_tests {
        use super::*;

        fn args(flags: &[&str]) -> Vec<OsString> {
            flags.iter().map(OsString::from).collect()
        }

        fn solver(flags: &[&str], base: CbmcSolver) -> Option<String> {
            effective_solver(&args(flags), &base)
        }

        fn some(name: &str) -> Option<String> {
            Some(name.to_string())
        }

        #[test]
        fn no_override_uses_base_solver() {
            assert_eq!(solver(&[], CbmcSolver::Cadical), some("cadical"));
            assert_eq!(solver(&[], CbmcSolver::Z3), some("z3"));
            assert_eq!(solver(&[], CbmcSolver::Binary("/opt/sat".to_string())), some("/opt/sat"));
        }

        #[test]
        fn minisat_emits_no_flag_and_is_cbmcs_default() {
            assert_eq!(solver(&[], CbmcSolver::Minisat), some("minisat"));
        }

        #[test]
        fn named_smt_override_wins_over_sat_base() {
            assert_eq!(solver(&["--z3"], CbmcSolver::Cadical), some("z3"));
        }

        #[test]
        fn named_smt_priority_is_order_independent() {
            for (higher, lower) in [
                ("--bitwuzla", "--boolector"),
                ("--boolector", "--cprover-smt2"),
                ("--cprover-smt2", "--mathsat"),
                ("--mathsat", "--cvc5"),
                ("--cvc5", "--yices"),
                ("--yices", "--z3"),
                ("--z3", "--smt2"),
            ] {
                let forward = solver(&[higher, lower], CbmcSolver::Cadical);
                let backward = solver(&[lower, higher], CbmcSolver::Cadical);
                assert_eq!(forward, backward, "{higher} vs {lower} must not depend on order");
                assert_eq!(forward, solver(&[higher], CbmcSolver::Cadical), "{higher} outranks");
            }
        }

        #[test]
        fn named_smt_solvers_resolve_to_their_names() {
            for (flag, name) in [
                ("--boolector", "boolector"),
                ("--cprover-smt2", "cprover-smt2"),
                ("--mathsat", "mathsat"),
                ("--yices", "yices"),
            ] {
                assert_eq!(solver(&[flag], CbmcSolver::Cadical), some(name));
            }
        }

        #[test]
        fn bare_smt2_lets_cbmc_choose() {
            assert_eq!(solver(&["--smt2"], CbmcSolver::Cadical), None);
        }

        #[test]
        fn unrelated_cbmc_arg_does_not_affect_resolution() {
            assert_eq!(solver(&["--object-bits", "16"], CbmcSolver::Kissat), some("kissat"));
        }

        #[test]
        fn sat_flags_take_their_value() {
            assert_eq!(solver(&["--sat-solver", "kissat"], CbmcSolver::Minisat), some("kissat"));
            assert_eq!(
                solver(&["--external-sat-solver", "/usr/bin/mysat"], CbmcSolver::Minisat),
                some("/usr/bin/mysat")
            );
        }

        /// Kani's own `--sat-solver cadical` comes first, so a `--cbmc-args --sat-solver` does
        /// nothing unless the base solver emits no such flag.
        #[test]
        fn cbmc_args_sat_solver_is_a_no_op_against_a_real_base_sat_flag() {
            assert_eq!(solver(&["--sat-solver", "kissat"], CbmcSolver::Cadical), some("cadical"));
        }

        #[test]
        fn smt_flag_outranks_sat_solver_either_order() {
            let flags = ["--sat-solver", "cadical", "--z3"];
            assert_eq!(solver(&flags, CbmcSolver::Cadical), some("z3"));
            let flags = ["--z3", "--sat-solver", "cadical"];
            assert_eq!(solver(&flags, CbmcSolver::Cadical), some("z3"));
        }

        #[test]
        fn external_sat_solver_outranks_sat_solver_either_order() {
            let flags = ["--sat-solver", "cadical", "--external-sat-solver", "/bin/kissat"];
            assert_eq!(solver(&flags, CbmcSolver::Cadical), some("/bin/kissat"));
            let flags = ["--external-sat-solver", "/bin/kissat", "--sat-solver", "cadical"];
            assert_eq!(solver(&flags, CbmcSolver::Cadical), some("/bin/kissat"));
        }

        #[test]
        fn repeated_sat_solver_first_occurrence_wins() {
            let flags = ["--sat-solver", "cadical", "--sat-solver", "kissat"];
            assert_eq!(solver(&flags, CbmcSolver::Minisat), some("cadical"));
            let flags = ["--external-sat-solver", "/bin/a", "--external-sat-solver", "/bin/b"];
            assert_eq!(solver(&flags, CbmcSolver::Minisat), some("/bin/a"));
        }

        #[test]
        fn base_smt_flag_beats_lower_priority_cbmc_args_flag() {
            assert_eq!(solver(&["--z3"], CbmcSolver::Bitwuzla), some("bitwuzla"));
        }

        #[test]
        fn cbmc_args_flag_beats_lower_priority_base_flag() {
            assert_eq!(solver(&["--bitwuzla"], CbmcSolver::Z3), some("bitwuzla"));
        }

        /// A named SMT solver flag keeps its dialect, but `--external-smt2-solver` supplies the
        /// executable CBMC runs, in either argument order and whether the named flag comes from
        /// Kani's own `--solver` or from `--cbmc-args`.
        #[test]
        fn external_smt_executable_is_exported_instead_of_the_named_solver() {
            let path = "/opt/custom-z3";
            let after = ["--z3", "--external-smt2-solver", path];
            let before = ["--external-smt2-solver", path, "--z3"];
            assert_eq!(solver(&after, CbmcSolver::Cadical), some(path));
            assert_eq!(solver(&before, CbmcSolver::Cadical), some(path));
            assert_eq!(solver(&["--external-smt2-solver", path], CbmcSolver::Z3), some(path));
            assert_eq!(solver(&["--external-smt2-solver", path], CbmcSolver::Cvc5), some(path));
        }

        #[test]
        fn repeated_external_smt_executable_first_occurrence_wins() {
            let flags =
                ["--z3", "--external-smt2-solver", "/bin/a", "--external-smt2-solver", "/bin/b"];
            assert_eq!(solver(&flags, CbmcSolver::Cadical), some("/bin/a"));
        }

        #[test]
        fn incremental_smt_executable_is_exported() {
            let flags = ["--incremental-smt2-solver", "/usr/bin/myincsmt"];
            assert_eq!(solver(&flags, CbmcSolver::Minisat), some("/usr/bin/myincsmt"));
        }
    }

    mod resolve_effective_unwind_tests {
        use super::*;
        use crate::metadata::tests::mock_proof_harness;
        use clap::Parser;

        fn cbmc_args(flags: &[&str]) -> Vec<OsString> {
            flags.iter().map(OsString::from).collect()
        }

        fn verify_opts(extra: &[&str]) -> VerificationArgs {
            let mut argv = vec!["kani"];
            argv.extend_from_slice(extra);
            argv.push("x.rs");
            crate::args::StandaloneArgs::try_parse_from(argv).unwrap().verify_opts
        }

        /// No native bound at all, so `--cbmc-args --unwind 7` is the only `--unwind` CBMC sees.
        #[test]
        fn raw_only_cbmc_args_unwind_applies_when_no_native_bound() {
            let h = mock_proof_harness("check_one", None, None, None);
            let resolved =
                resolve_effective_unwind(&verify_opts(&[]), &h, &cbmc_args(&["--unwind", "7"]));
            assert_eq!(resolved, Some(7));
        }

        #[test]
        fn no_native_bound_and_no_raw_unwind_is_null() {
            let h = mock_proof_harness("check_one", None, None, None);
            let resolved = resolve_effective_unwind(&verify_opts(&[]), &h, &cbmc_args(&[]));
            assert_eq!(resolved, None);
        }

        #[test]
        fn native_bound_beats_raw_cbmc_args_unwind() {
            let h = mock_proof_harness("check_one", None, None, None);
            let resolved = resolve_effective_unwind(
                &verify_opts(&["--unwind", "3", "--harness", "check_one"]),
                &h,
                &cbmc_args(&["--unwind", "99"]),
            );
            assert_eq!(resolved, Some(3));
        }

        #[test]
        fn first_of_two_raw_cbmc_args_unwind_wins() {
            let h = mock_proof_harness("check_one", None, None, None);
            let resolved = resolve_effective_unwind(
                &verify_opts(&[]),
                &h,
                &cbmc_args(&["--unwind", "10", "--unwind", "3"]),
            );
            assert_eq!(resolved, Some(10));
            let resolved = resolve_effective_unwind(
                &verify_opts(&[]),
                &h,
                &cbmc_args(&["--unwind", "3", "--unwind", "10"]),
            );
            assert_eq!(resolved, Some(3));
        }

        #[test]
        fn harness_attribute_bound_counts_as_native() {
            let h = mock_proof_harness("check_one", Some(4), None, None);
            let resolved =
                resolve_effective_unwind(&verify_opts(&[]), &h, &cbmc_args(&["--unwind", "99"]));
            assert_eq!(resolved, Some(4));
        }

        #[test]
        fn unparsable_raw_unwind_value_is_null_not_a_panic() {
            let h = mock_proof_harness("check_one", None, None, None);
            let resolved = resolve_effective_unwind(
                &verify_opts(&[]),
                &h,
                &cbmc_args(&["--unwind", "not-a-number"]),
            );
            assert_eq!(resolved, None);
        }
    }

    #[test]
    fn export_with_other_failure_kind() {
        let h = harness("other_failure_harness");
        let properties = vec![property("assertion", 1, CheckStatus::Failure)];
        let mut result = success_result(properties);
        result.status = VerificationStatus::Failure;
        result.failed_properties = FailedProperties::Other;
        let hr = HarnessResult { harness: &h, result };

        let v = serde_json::to_value(export_one(hr)).unwrap();
        assert_eq!(v["harnesses"][0]["failure_kind"], "OTHER");
    }

    #[test]
    fn export_autoharness_generated_flag_is_true_for_generated_harness() {
        let mut h = harness("generated_from_add_numbers");
        h.is_automatically_generated = true;
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v = serde_json::to_value(export_one(hr)).unwrap();
        assert_eq!(v["harnesses"][0]["is_automatically_generated"], true);
    }

    #[test]
    fn export_is_bounded_true_iff_autoharness_bounded_argument() {
        let mut h_bounded = harness("bounded_autoharness");
        h_bounded.is_automatically_generated = true;
        h_bounded.is_bounded = true;
        let hr_bounded = HarnessResult { harness: &h_bounded, result: success_result(vec![]) };
        let v_bounded = serde_json::to_value(export_one(hr_bounded)).unwrap();
        assert_eq!(v_bounded["harnesses"][0]["is_bounded"], true);

        let h_unbounded = harness("manual_harness");
        let hr_unbounded = HarnessResult { harness: &h_unbounded, result: success_result(vec![]) };
        let v_unbounded = serde_json::to_value(export_one(hr_unbounded)).unwrap();
        assert_eq!(v_unbounded["harnesses"][0]["is_bounded"], false);
    }

    #[test]
    fn export_contract_null_and_present() {
        let h_none = harness("no_contract_harness");

        let mut h_plain = harness("plain_contract_harness");
        h_plain.contract = Some(AssignsContract {
            contracted_function_name: "target_fn".to_string(),
            recursion_tracker: None,
        });

        let mut h_recursive = harness("recursive_contract_harness");
        h_recursive.contract = Some(AssignsContract {
            contracted_function_name: "recursive_target_fn".to_string(),
            recursion_tracker: Some("KANI_RECURSION_recursive_target_fn".to_string()),
        });

        let hrs = [
            HarnessResult { harness: &h_none, result: success_result(vec![]) },
            HarnessResult { harness: &h_plain, result: success_result(vec![]) },
            HarnessResult { harness: &h_recursive, result: success_result(vec![]) },
        ];
        let ctx = RunContext {
            harness_selection: HarnessSelectionExport {
                requested_filters: Vec::new(),
                exact: false,
                unmatched_filters: Vec::new(),
                matched_count: 3,
            },
            ..test_context()
        };
        let v = serde_json::to_value(export_with(&hrs, ctx)).unwrap();

        let by_name = |name: &str| -> serde_json::Value {
            v["harnesses"].as_array().unwrap().iter().find(|h| h["name"] == name).unwrap().clone()
        };

        assert!(by_name("no_contract_harness")["contract"].is_null());

        let plain = by_name("plain_contract_harness");
        assert_eq!(plain["contract"]["contracted_function_name"], "target_fn");
        assert!(plain["contract"]["recursion_tracker"].is_null());

        let recursive = by_name("recursive_contract_harness");
        assert_eq!(recursive["contract"]["contracted_function_name"], "recursive_target_fn");
        assert_eq!(
            recursive["contract"]["recursion_tracker"],
            "KANI_RECURSION_recursive_target_fn"
        );
    }

    #[test]
    fn export_warnings_passthrough() {
        let h = harness("h");
        let mut result = success_result(vec![]);
        result.warnings = vec![Warning {
            message: "ignoring forall".to_string(),
            truncated: false,
            original_chars: None,
        }];
        let hr = HarnessResult { harness: &h, result };
        let v = serde_json::to_value(export_one(hr)).unwrap();
        assert_eq!(v["harnesses"][0]["warnings"][0]["message"], "ignoring forall");
        assert_eq!(v["harnesses"][0]["warnings"][0]["truncated"], false);
        assert!(v["harnesses"][0]["warnings"][0]["original_chars"].is_null());
        assert_eq!(v["harnesses"][0]["warnings_truncated"], 0);
    }

    fn dir_entry_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn write_json_atomically_overwrites_stale_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export.json");
        std::fs::write(&path, b"stale content from an earlier, crashed run").unwrap();

        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let export = export_one(hr);
        write_json_atomically(&path, &export).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(!contents.contains("stale content"));
        assert!(contents.contains("\"schema_version\""));
        assert_eq!(
            dir_entry_names(dir.path()),
            vec!["export.json".to_string()],
            "no temp file may survive a successful export"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_json_atomically_creates_the_file_with_the_mode_std_fs_write_would() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let reference = dir.path().join("reference");
        std::fs::write(&reference, b"").unwrap();
        let target = dir.path().join("export.json");

        write_json_atomically(&target, &serde_json::json!({})).unwrap();

        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode();
        assert_eq!(mode(&target), mode(&reference));
    }

    #[cfg(unix)]
    #[test]
    fn write_json_atomically_keeps_the_mode_of_an_existing_target() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("export.json");
        for mode in [0o600, 0o640, 0o666] {
            std::fs::write(&target, b"stale").unwrap();
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode)).unwrap();

            write_json_atomically(&target, &serde_json::json!({})).unwrap();

            let actual = std::fs::metadata(&target).unwrap().permissions().mode() & 0o7777;
            assert_eq!(actual, mode, "mode {mode:o} of the existing target was not kept");
        }
    }

    #[test]
    fn write_json_atomically_succeeds_with_max_length_destination_basename() {
        let dir = tempfile::tempdir().unwrap();
        let long_stem = "e".repeat(255 - ".json".len());
        let path = dir.path().join(format!("{long_stem}.json"));
        assert_eq!(path.file_name().unwrap().len(), 255);

        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let export = export_one(hr);
        write_json_atomically(&path, &export).unwrap();

        assert!(path.exists());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"schema_version\""));
    }

    #[test]
    fn write_json_atomically_succeeds_when_no_stale_file_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fresh").join("export.json");

        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let export = export_one(hr);
        write_json_atomically(&path, &export).unwrap();

        assert!(path.exists());
        assert_eq!(dir_entry_names(path.parent().unwrap()), vec!["export.json".to_string()]);
    }

    #[test]
    fn write_json_atomically_fails_deterministically_when_parent_path_is_a_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let not_a_dir = dir.path().join("not-a-dir");
        std::fs::write(&not_a_dir, b"a regular file, not a directory").unwrap();
        let path = not_a_dir.join("nested").join("export.json");

        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let export = export_one(hr);

        let result = write_json_atomically(&path, &export);

        assert!(
            result.is_err(),
            "a parent path component that is a regular file must fail deterministically"
        );
        assert!(!path.exists(), "no target can exist under a non-directory parent");
    }

    /// A value that fails after enough output to overflow `BufWriter`, so bytes have already
    /// reached the temporary file when the error occurs.
    struct FailsMidWrite;

    impl Serialize for FailsMidWrite {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::{Error, SerializeStruct};
            let mut state = serializer.serialize_struct("FailsMidWrite", 1)?;
            state.serialize_field("padding", &"x".repeat(64 * 1024))?;
            Err(S::Error::custom("serialization failed"))
        }
    }

    #[test]
    fn write_json_atomically_preserves_sentinel_file_when_serialization_fails() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("export.json");
        let sentinel: &[u8] = b"sentinel content from an earlier run, do not touch";
        std::fs::write(&target, sentinel).unwrap();

        let err = write_json_atomically(&target, &FailsMidWrite).unwrap_err();

        let message = format!("{err:#}");
        assert!(message.contains("serialization failed"), "{message}");
        assert!(message.contains("export.json"), "the error must name the target: {message}");
        assert_eq!(std::fs::read(&target).unwrap(), sentinel);
        assert_eq!(
            dir_entry_names(dir.path()),
            vec!["export.json".to_string()],
            "a failed write must leave no temp file behind either"
        );
    }

    #[test]
    fn write_json_atomically_preserves_existing_target_when_persist_fails() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("export.json");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("marker.txt"), b"keep me").unwrap();

        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let export = export_one(hr);

        let result = write_json_atomically(&target, &export);

        assert!(result.is_err(), "persisting onto an existing directory must fail");
        assert!(target.is_dir(), "the pre-existing target must survive a failed write");
        assert!(
            target.join("marker.txt").exists(),
            "no partial write may clobber the pre-existing target after a failed persist"
        );
        assert_eq!(
            dir_entry_names(dir.path()),
            vec!["export.json".to_string()],
            "a failed persist must leave no temp file behind either"
        );
    }

    #[test]
    fn export_parent_dir_resolves_bare_and_rootless_paths_to_cwd() {
        assert_eq!(export_parent_dir(Path::new("/a/b/export.json")), Path::new("/a/b"));
        assert_eq!(export_parent_dir(Path::new("export.json")), Path::new("."));
        assert_eq!(export_parent_dir(Path::new("/")), Path::new("."));
    }

    #[test]
    fn export_json_path_dash_is_literal_filename() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("-");

        let h = harness("h");
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let export = export_one(hr);
        write_json_atomically(&path, &export).unwrap();

        assert!(path.exists(), "a path literally named `-` must be written like any other file");
        assert!(path.is_file());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"schema_version\""));
    }

    #[test]
    fn export_file_is_repo_relative() {
        let mut h = harness("h");
        h.original_file =
            std::env::current_dir().unwrap().join("src/foo.rs").to_string_lossy().into_owned();
        let hr = HarnessResult { harness: &h, result: success_result(vec![]) };
        let v = serde_json::to_value(export_one(hr)).unwrap();
        assert_eq!(v["harnesses"][0]["file"], "src/foo.rs");
    }
}
