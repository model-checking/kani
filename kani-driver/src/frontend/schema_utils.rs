// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Utility functions for creating structured JSON schemas
// This module contains helper functions to convert Kani internal structures to JSON

use crate::call_cbmc::VerificationStatus;
use crate::cbmc_output_parser::{CheckStatus, Property};
use crate::frontend::JsonHandler;
use crate::harness_runner::HarnessResult;
use crate::project::Project;
use crate::session::KaniSession;
use anyhow::Result;
use kani_metadata::{CbmcSolver, HarnessMetadata};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::process::Command;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Creates structured JSON metadata for an export run
/// This utility function captures basic environment for the whole session
pub fn create_metadata_json() -> Value {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    let kani_version = env!("CARGO_PKG_VERSION");
    let target = env!("TARGET");
    let build_mode = if cfg!(debug_assertions) { "debug" } else { "release" };

    json!({
    "version": "1.0",
    "timestamp": timestamp,
    "kani_version": kani_version,
    "target": target,
    "build_mode": build_mode,
    })
}

/// Ask a tool for its version, returning the first line of its `--version` output.
///
/// Returns `None` if the tool cannot be run or says nothing: a version we could not determine is
/// reported as null rather than guessed, and never fails the run.
fn tool_version(binary: &OsStr) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?.trim();
    (!first_line.is_empty()).then(|| first_line.to_string())
}

/// The version of every tool this run relies on, addressing part of
/// <https://github.com/model-checking/kani/issues/942>'s sibling request
/// <https://github.com/model-checking/kani/issues/2572>.
///
/// A key is present only if the run uses that tool, so an absent key means "not part of this run"
/// while a present null means "used, but its version could not be determined". The versions are
/// verbatim first lines of `--version` output, so they are display strings rather than parseable
/// values.
///
/// This spawns a process per tool. That is only paid when `--export-json` is requested, and once per
/// run rather than per harness.
pub fn create_tool_versions_json(session: &KaniSession, harnesses: &[&HarnessMetadata]) -> Value {
    let mut tools = serde_json::Map::new();

    tools.insert("kani".to_string(), json!(env!("CARGO_PKG_VERSION")));

    // `kani-compiler` is a rustc driver, so `--version` reports the toolchain it was built against.
    // That is the version that decides how Rust is translated, which makes it the interesting one.
    // Asking the binary beats reading a `rustc` from PATH, which need not be the same toolchain.
    tools.insert("rustc".to_string(), json!(tool_version(session.kani_compiler.as_os_str())));

    // The CBMC suite. `cbmc` and `goto-cc` run for every verification; `goto-instrument` runs to
    // instrument the goto program; `goto-synthesizer` only for loop-contract synthesis.
    for (key, binary) in
        [("cbmc", "cbmc"), ("goto_cc", "goto-cc"), ("goto_instrument", "goto-instrument")]
    {
        tools.insert(key.to_string(), json!(tool_version(OsStr::new(binary))));
    }
    if session.args.synthesize_loop_contracts {
        tools.insert(
            "goto_synthesizer".to_string(),
            json!(tool_version(OsStr::new("goto-synthesizer"))),
        );
    }

    // Solvers, which can differ per harness, so this is every distinct solver the run resolves to.
    // A list rather than a map keyed by name, because the set varies per run and consumers should
    // not have to discover which keys might appear. Only solvers CBMC invokes as separate binaries
    // have a version of their own: CaDiCaL and MiniSAT are built into CBMC and would report its
    // version, so they are named with a null version rather than given a misleading one.
    let mut solvers = BTreeMap::new();
    for harness in harnesses {
        let solver = effective_solver(session, &harness.attributes.solver);
        // A run whose solver CBMC chooses for itself names no solver to report.
        let Some(name) = solver.name else { continue };
        // Probe each binary once, however many harnesses use it. Ordered by name so two runs with
        // the same solvers produce the same document.
        solvers
            .entry(name)
            .or_insert_with(|| solver.binary.and_then(|binary| tool_version(OsStr::new(&binary))));
    }
    tools.insert(
        "solvers".to_string(),
        json!(
            solvers
                .into_iter()
                .map(|(name, version)| json!({"name": name, "version": version}))
                .collect::<Vec<_>>()
        ),
    );

    Value::Object(tools)
}

/// Every property of a harness, counted by status.
///
/// The counts partition the properties exhaustively, so
/// `passed + failed + unreachable + undetermined + error + satisfied + unsatisfiable + covered +
/// uncovered == total_properties` always holds. Counting only a few statuses meant the numbers
/// silently failed to reconcile whenever a run had cover statements, coverage properties, or a
/// solver error, and a consumer had no way to tell that from a run where they genuinely summed.
///
/// The match below is deliberately exhaustive: a new `CheckStatus` should fail to compile here
/// rather than quietly go uncounted.
#[derive(Default)]
struct PropertyCounts {
    passed: usize,
    failed: usize,
    unreachable: usize,
    undetermined: usize,
    error: usize,
    satisfied: usize,
    unsatisfiable: usize,
    covered: usize,
    uncovered: usize,
}

impl PropertyCounts {
    fn of(properties: &[Property]) -> Self {
        let mut counts = Self::default();
        for property in properties {
            let counter = match property.status {
                CheckStatus::Success => &mut counts.passed,
                CheckStatus::Failure => &mut counts.failed,
                CheckStatus::Unreachable => &mut counts.unreachable,
                // Kani renders both of these as UNDETERMINED, so they are grouped here to keep the
                // export agreeing with the text output. CBMC 6+ reports UNKNOWN when another
                // property's failure makes this one impossible to conclude either way.
                CheckStatus::Undetermined | CheckStatus::Unknown => &mut counts.undetermined,
                CheckStatus::Error => &mut counts.error,
                CheckStatus::Satisfied => &mut counts.satisfied,
                CheckStatus::Unsatisfiable => &mut counts.unsatisfiable,
                CheckStatus::Covered => &mut counts.covered,
                CheckStatus::Uncovered => &mut counts.uncovered,
            };
            *counter += 1;
        }
        counts
    }

    fn to_json(&self) -> Value {
        json!({
            "total_properties": self.passed + self.failed + self.unreachable + self.undetermined
                + self.error + self.satisfied + self.unsatisfiable + self.covered + self.uncovered,
            "passed": self.passed,
            "failed": self.failed,
            "unreachable": self.unreachable,
            "undetermined": self.undetermined,
            "solver_error": self.error,
            "satisfied": self.satisfied,
            "unsatisfiable": self.unsatisfiable,
            "covered": self.covered,
            "uncovered": self.uncovered,
        })
    }

    /// The same shape, for a harness whose properties were never measured, with a caller-supplied
    /// explanation of why (e.g. a CBMC failure vs. never having run at all).
    fn unmeasured_json_with_reason(reason: &str) -> Value {
        json!({
            "total_properties": null,
            "passed": null,
            "failed": null,
            "unreachable": null,
            "undetermined": null,
            "solver_error": null,
            "satisfied": null,
            "unsatisfiable": null,
            "covered": null,
            "uncovered": null,
            "error": reason
        })
    }

    /// The same shape, for a harness whose properties were never measured.
    fn unmeasured_json() -> Value {
        Self::unmeasured_json_with_reason(
            "Could not extract property details due to verification failure",
        )
    }
}

/// Creates structured JSON metadata for the project
/// This utility function captures detailed info for the project
pub fn create_project_metadata_json(project: &Project) -> Value {
    json!({
    "crate_name": project.metadata.iter()
    .map(|m| m.crate_name.clone())
    .collect::<Vec<String>>(),
    // The real workspace root, which only exists for Cargo projects; null for a standalone run.
    // `Project::outdir` is the compiler output directory -- for Cargo it sits under
    // `target/<triple>/debug/deps` -- so reporting it as the workspace root was simply wrong.
    "workspace_root": project.cargo_metadata.as_ref()
    .map(|metadata| metadata.workspace_root.clone()),
    "output_dir": project.outdir.clone(),
    })
}
/// Creates structured JSON metadata for a harness
/// This utility function separates harness metadata creation from the main verification logic
pub fn create_harness_metadata_json(h: &HarnessMetadata) -> Value {
    json!({
        "pretty_name": h.pretty_name, // use this as identifier
        "mangled_name": h.mangled_name,
        "crate_name": h.crate_name,
        "source": {
            "file": h.original_file,
            "start_line": h.original_start_line,
            "end_line": h.original_end_line
        },
        "goto_file": h.goto_file.as_ref().map(|p| p.to_string_lossy().to_string()),
        "attributes": {
            "kind": format!("{:?}", h.attributes.kind),
            "should_panic": h.attributes.should_panic,
        },
        "contract":{
            "contracted_function_name": h.contract.as_ref()
            .map(|c| c.contracted_function_name.as_str()),
            "recursion_tracker": h.contract.as_ref()
            .and_then(|c| c.recursion_tracker.as_ref()),
        },
        "has_loop_contracts": h.has_loop_contracts,
        "is_automatically_generated": h.is_automatically_generated,

    })
}

/// Creates verification result JSON with harness reference
/// This reduces duplication between harness metadata and verification results
pub fn create_verification_result_json(result: &HarnessResult) -> Value {
    // Extract detailed verification results as "checks"
    let checks = match &result.result.results {
        Ok(properties) => properties
            .iter()
            .enumerate()
            .map(|(i, prop)| {
                json!({
                    "id": i + 1,
                    "function": prop.property_id.fn_name.as_deref().unwrap_or("unknown"),
                    "status": format!("{:?}", prop.status),
                    "description": prop.description,
                    "location": {
                        "file": prop.source_location.file.as_deref().unwrap_or("unknown"),
                        "line": prop.source_location.line.as_deref().unwrap_or("unknown"),
                        "column": prop.source_location.column.as_deref().unwrap_or("unknown"),
                    },
                    "category": prop.property_id.class,
                })
            })
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    json!({
        "harness_id": result.harness.pretty_name,  // Reference to harness instead of duplicating name
        "status": match result.result.status {
            VerificationStatus::Success => "Success",
            VerificationStatus::Failure => "Failure",
        },
        "duration_ms": (result.result.runtime.as_millis() as u64),
        "checks": checks,
    })
}

/// Creates a verification summary with clean structure
pub fn create_verification_summary_json(
    results: &[HarnessResult],
    selected: usize,
    status_label: &str,
) -> Value {
    let successful =
        results.iter().filter(|r| r.result.status == VerificationStatus::Success).count();
    let failed = results.len() - successful;
    let total_duration_ms: u64 = results.iter().map(|r| r.result.runtime.as_millis() as u64).sum();

    let verification_results: Vec<_> =
        results.iter().map(|r| create_verification_result_json(r)).collect();

    json!({
        "summary": {
            "total_harnesses": selected,
            "executed": results.len(),
            "status": status_label,
            "successful": successful,
            "failed": failed,
            "duration_ms": total_duration_ms
        },
        "results": verification_results
    })
}

/// Helper function to add verification results to JsonHandler
/// This utility function encapsulates the logic for adding verification summary to JSON output
pub fn add_runner_results_to_json(
    handler: &mut JsonHandler,
    results: &[HarnessResult],
    selected: usize,
    status_label: &str,
) {
    // Use frontend utility to create structured verification summary
    let summary = create_verification_summary_json(results, selected, status_label);
    handler.add_item("verification_results", summary);
}

/// Process harness results and enrich JSON handler with additional metadata.
/// This function handles the complex harness processing logic, combining verification results
/// with harness metadata to create enriched JSON output.
pub fn process_harness_results(
    handler: &mut JsonHandler,
    harnesses: &[&HarnessMetadata],
    results: &[HarnessResult],
) -> Result<()> {
    // The main verification results are handled by the harness runner
    for h in harnesses {
        // Joined on `mangled_name`, the unique identifier `harness_metadata` already carries,
        // rather than `pretty_name`: two harnesses in different crates of the same workspace can
        // share a `pretty_name`, and joining on that would attribute a result -- including a
        // failure -- to the wrong harness. `harness_id` in the emitted JSON is unchanged; only
        // the join predicate used to find the matching result moves to the unique key.
        let harness_result = results.iter().find(|r| r.harness.mangled_name == h.mangled_name);

        // Add error details for this harness. This accumulates one entry per harness, keyed by
        // `harness_id`, the same way the `cbmc` array does: a single top-level object would let a
        // later successful harness overwrite an earlier failure, and report a run containing a
        // failing harness as having no errors at all.
        if let Some(result) = harness_result {
            handler.add_harness_detail("error_details", match result.result.status {
                VerificationStatus::Failure => {
                    json!({
                        "harness_id": h.pretty_name,
                        "has_errors": true,
                        "error_type": match result.result.failed_properties {
                            crate::call_cbmc::FailedProperties::None => "unknown_failure",
                            crate::call_cbmc::FailedProperties::PanicsOnly => "assertion_failure",
                            crate::call_cbmc::FailedProperties::Other => "verification_failure",
                            crate::call_cbmc::FailedProperties::Error => "property_error",
                        },
                        "failed_properties_type": format!("{:?}", result.result.failed_properties),
                        "exit_status": match &result.result.results {
                            Err(crate::call_cbmc::ExitStatus::Timeout) => "timeout".to_string(),
                            Err(crate::call_cbmc::ExitStatus::OutOfMemory) => "out_of_memory".to_string(),
                            Err(crate::call_cbmc::ExitStatus::Other(code)) => format!("exit_code_{}", code),
                            Ok(_) => "properties_failed".to_string()
                        }
                    })
                },
                VerificationStatus::Success => json!({
                    "harness_id": h.pretty_name,
                    "has_errors": false
                })
            });

            // Add property details for this harness. `harness_id` is what makes an entry
            // attributable: this array is built in harness-metadata order while
            // `verification_results.results` is in completion order, so the two cannot be
            // correlated by position.
            handler.add_harness_detail(
                "property_details",
                json!({
                    "harness_id": h.pretty_name,
                    "property_details": match &result.result.results {
                        Ok(properties) => PropertyCounts::of(properties).to_json(),
                        // CBMC produced no property results at all (timeout, out of memory, crash).
                        // Keep every count present so the shape does not change between runs, and
                        // report them as null: `0` would assert that nothing failed, when the truth is
                        // that nothing was measured.
                        Err(_) => PropertyCounts::unmeasured_json()
                    }
                }),
            );
        } else {
            // This harness was selected (it has a `harness_metadata` entry) but has no entry in
            // `results`. That is not always "never ran": under `--fail-fast`,
            // `check_all_harnesses` collects harness futures into a single `Result<Vec<_>>`, and
            // as soon as one harness fails, the whole collection short-circuits on that `Err` --
            // discarding the `Ok` results of any other harness that had already completed
            // (including a pass) but lost the race to be collected before the failure. So a
            // harness landing in this branch may have genuinely been skipped, or may have run
            // and even passed, with its result simply not retained. Without this branch the
            // harness would be silently absent from both `error_details` and `property_details`,
            // which a consumer correlating those arrays against `harness_metadata` (or checking
            // "every detail entry is a Success") could easily misread as "nothing wrong with it".
            // "skipped"/"not_run" would overclaim the former case for certain, so this reports
            // the honest, disjunctive truth instead.
            handler.add_harness_detail(
                "error_details",
                json!({
                    "harness_id": h.pretty_name,
                    "has_errors": true,
                    "error_type": "not_reported",
                    "exit_status": "unknown"
                }),
            );

            handler.add_harness_detail(
                "property_details",
                json!({
                    "harness_id": h.pretty_name,
                    "property_details": PropertyCounts::unmeasured_json_with_reason(
                        "No result was reported for this harness (e.g. skipped after \
                         --fail-fast, or a completed result not retained)."
                    )
                }),
            );
        }
    }

    Ok(())
}

/// The solver CBMC will actually run with.
struct EffectiveSolver {
    /// Display name, or `None` when the choice is left to CBMC -- `--smt2` on its own names no
    /// solver, and a wrong name is worse than no name.
    name: Option<String>,
    /// The binary to ask for a version, when CBMC runs the solver as a separate process. `None` for
    /// solvers built into CBMC, which would report CBMC's own version rather than one of their own.
    binary: Option<String>,
}

/// Resolve the solver for a harness, applying every layer that can select one.
///
/// `KaniSession::resolved_solver` covers `--solver`, the harness attribute and the default, but
/// `--cbmc-args` is appended *after* Kani's own solver flags and CBMC takes the last one it sees, so
/// a solver named there overrides all three. Everything that reports a solver must go through here,
/// or the run's own metadata ends up contradicting itself.
fn effective_solver(session: &KaniSession, harness_solver: &Option<CbmcSolver>) -> EffectiveSolver {
    let mut override_seen = false;
    let mut resolved = EffectiveSolver { name: None, binary: None };
    let mut cbmc_args = session.args.cbmc_args.iter();
    while let Some(arg) = cbmc_args.next() {
        // Last one wins, matching CBMC.
        let (name, binary) = match arg.to_str() {
            // `--sat-solver` selects a solver built into CBMC, so there is no binary to probe.
            Some("--sat-solver") => {
                (cbmc_args.next().and_then(|name| name.to_str()).map(str::to_string), None)
            }
            Some("--external-sat-solver") => {
                let binary = cbmc_args.next().and_then(|name| name.to_str()).map(str::to_string);
                (binary.clone(), binary)
            }
            Some("--bitwuzla") => (Some("bitwuzla".to_string()), Some("bitwuzla".to_string())),
            Some("--cvc5") => (Some("cvc5".to_string()), Some("cvc5".to_string())),
            Some("--z3") => (Some("z3".to_string()), Some("z3".to_string())),
            // `--smt2` alone leaves the choice of SMT solver to CBMC.
            Some("--smt2") => (None, None),
            _ => continue,
        };
        override_seen = true;
        resolved = EffectiveSolver { name, binary };
    }

    if override_seen {
        return resolved;
    }

    let (name, binary) = match session.resolved_solver(harness_solver) {
        CbmcSolver::Bitwuzla => ("bitwuzla", Some("bitwuzla")),
        CbmcSolver::Cadical => ("cadical", None),
        CbmcSolver::Cvc5 => ("cvc5", Some("cvc5")),
        CbmcSolver::Kissat => ("kissat", Some("kissat")),
        CbmcSolver::Minisat => ("minisat", None),
        CbmcSolver::Z3 => ("z3", Some("z3")),
        CbmcSolver::Binary(binary) => (binary.as_str(), Some(binary.as_str())),
    };
    EffectiveSolver { name: Some(name.to_string()), binary: binary.map(str::to_string) }
}

/// The `--object-bits` value CBMC will actually run with.
///
/// `VerificationArgs::cbmc_object_bits` reports nothing once the user supplies `--object-bits`
/// through `--cbmc-args`, because Kani then stops passing its own default. That is the right answer
/// for building the command line and the wrong one for describing it: the run does have a value, so
/// take it from `--cbmc-args` instead of exporting null for an explicitly configured run.
fn effective_object_bits(session: &KaniSession) -> Option<u32> {
    if let Some(bits) = session.args.cbmc_object_bits() {
        return Some(bits);
    }
    let mut cbmc_args = session.args.cbmc_args.iter();
    while let Some(arg) = cbmc_args.next() {
        if arg == "--object-bits" {
            return cbmc_args.next().and_then(|bits| bits.to_str()?.parse().ok());
        }
    }
    None
}

pub fn process_cbmc_results(
    handler: &mut JsonHandler,
    harnesses: &[&HarnessMetadata],
    results: &[HarnessResult],
    session: &KaniSession,
) -> Result<()> {
    let cbmc_info_opt = session.get_cbmc_info().ok();
    for h in harnesses {
        // See the matching comment in `process_harness_results`: join on the unique
        // `mangled_name` rather than `pretty_name`, which two harnesses in different crates of a
        // workspace can share.
        let harness_result = results.iter().find(|r| r.harness.mangled_name == h.mangled_name);
        handler.add_harness_detail("cbmc", json!({
            // basic name for harnesses
            "harness_id": h.pretty_name,

            // Per-harness CBMC info (key-value pairs) without parsing CBMC stdout
            "cbmc_metadata": {
                // Version / OS info (same for all harnesses in a run)
                "version": cbmc_info_opt.as_ref().map(|i| i.version.clone()),
                "os_info": cbmc_info_opt.as_ref().map(|i| i.os_info.clone()),
            },
            // Configuration passed to CBMC for this harness. Both values are resolved the same way
            // the CBMC command line itself resolves them, so that a consumer reading this is
            // reading the run that actually happened.
            "configuration": {
                "object_bits": effective_object_bits(session),
                "solver": effective_solver(session, &h.attributes.solver).name,
            },

            // CBMC execution statistics extracted from messages
            "cbmc_stats": harness_result.and_then(|r| r.result.cbmc_stats.as_ref()).map(|s| json!({
            "runtime_symex_s": s.runtime_symex_s,
            "size_program_expression": s.size_program_expression,
            "slicing_removed_assignments": s.slicing_removed_assignments,
            "vccs_generated": s.vccs_generated,
            "vccs_remaining": s.vccs_remaining,
            "runtime_postprocess_equation_s": s.runtime_postprocess_equation_s,
            "runtime_convert_ssa_s": s.runtime_convert_ssa_s,
            "runtime_post_process_s": s.runtime_post_process_s,
            "runtime_solver_s": s.runtime_solver_s,
            "runtime_decision_procedure_s": s.runtime_decision_procedure_s
            }))
        }));
    }
    Ok(())
}

/// Simple container to standardize tool outputs captured during verification
#[derive(Serialize)]
#[allow(dead_code)]
pub struct ToolOutput<'a> {
    /// Arbitrary tool name key under which this output will be grouped
    pub tool: &'a str,
    /// Harness identifier this output belongs to
    pub harness_id: &'a str,
    /// Unparsed stdout text emitted by the tool
    pub stdout: &'a str,
}

/// Add a tool output entry to the JSON under a tool-named array
#[allow(dead_code)]
pub fn add_tool_output(handler: &mut JsonHandler, output: ToolOutput<'_>) {
    // structure: top-level key is the tool name, value is an array of entries
    handler.add_harness_detail(
        output.tool,
        json!({
            "harness_id": output.harness_id,
            "stdout": output.stdout,
        }),
    );
}
