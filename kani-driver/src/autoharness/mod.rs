// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::str::FromStr;

use crate::args::Timeout;
use crate::args::VerificationArgs;
use crate::args::autoharness_args::{
    AutoharnessBounds, CargoAutoharnessArgs, CommonAutoharnessArgs, StandaloneAutoharnessArgs,
};
use crate::args::common::UnstableFeature;
use crate::call_cbmc::VerificationStatus;
use crate::harness_runner::HarnessResult;
use crate::list::collect_metadata::process_metadata;
use crate::list::output::output_list_results;
use crate::project::{Project, standalone_project, std_project};
use crate::session::KaniSession;
use crate::util::warning;
use crate::{InvocationType, codegen_llbc_only, print_kani_version, project, verify_project};
use anyhow::Result;
use comfy_table::Table as PrettyTable;
use kani_metadata::{AutoHarnessSkipReason, HarnessMetadata, KaniMetadata};

const AUTOHARNESS_TIMEOUT: &str = "60s";
const LOOP_UNWIND_DEFAULT: u32 = 20;

/// Printed after the summary table if some harness is marked "(refcount)", c.f.
/// `HarnessMetadata::is_refcount_bounded`. Deliberately states only that the modeled counts are
/// finite, not which ones: the representatives are an implementation detail of the `Rc`/`Arc`
/// generators (<https://github.com/model-checking/kani/issues/4752>).
const REFCOUNT_NOTE: &str = "Note: harnesses marked \"(refcount)\" generate Rc/Arc values whose reference counts are modeled with only finitely many states\n\
     (this needs no --bounded-arguments). Their verification results do not cover every reference count a caller could produce,\n\
     e.g. code that compares strong_count/weak_count against specific values may not be fully verified.";

/// The "Kind of Automatic Harness" column of the summary table: the harness kind, followed by a
/// marker for each caveat that restricts which inputs its verification result covers.
fn harness_kind_label(harness: &HarnessMetadata) -> String {
    let mut kind = harness.attributes.kind.to_string();
    if harness.is_bounded {
        kind.push_str(" (bounded)");
    }
    if harness.is_ctor_based {
        kind.push_str(" (ctor)");
    }
    if harness.is_refcount_bounded {
        kind.push_str(" (refcount)");
    }
    kind
}

pub fn autoharness_cargo(args: CargoAutoharnessArgs) -> Result<()> {
    let mut session = KaniSession::new(args.verify_opts)?;
    setup_session(&mut session, &args.common_autoharness_args);

    if !session.args.common_args.quiet {
        print_kani_version(InvocationType::CargoKani(vec![]), session.args.common_args.verbose);
    }
    let project = project::cargo_project(&mut session, false)?;
    postprocess_project(project, session, args.common_autoharness_args)
}

pub fn autoharness_standalone(args: StandaloneAutoharnessArgs) -> Result<()> {
    let mut session = KaniSession::new(args.verify_opts)?;
    setup_session(&mut session, &args.common_autoharness_args);

    if !session.args.common_args.quiet {
        print_kani_version(InvocationType::Standalone, session.args.common_args.verbose);
    }

    let project = if args.std {
        std_project(&args.input, &session)?
    } else {
        standalone_project(&args.input, args.crate_name, &session)?
    };

    postprocess_project(project, session, args.common_autoharness_args)
}

/// Execute autoharness-specific KaniSession configuration.
fn setup_session(session: &mut KaniSession, common_autoharness_args: &CommonAutoharnessArgs) {
    // `main` already applies these before validating the arguments (so that validation sees the
    // options the run will actually use); repeat it here -- the call is idempotent -- so that the
    // session is configured correctly regardless of how it was constructed.
    session.args.apply_autoharness_parallel_defaults();
    session.enable_autoharness();
    session.add_default_bounds();
    let bounds = common_autoharness_args.bounds();
    if common_autoharness_args.bounded_arguments {
        warn_if_bounds_reach_unwind(bounds, &session.args);
    }
    session.add_auto_harness_args(
        &common_autoharness_args.include_pattern,
        &common_autoharness_args.exclude_pattern,
        common_autoharness_args.bounded_arguments,
        common_autoharness_args.constructor_args,
        common_autoharness_args.check_invariants,
        bounds,
    );
}

/// Warn about a bound that reaches the effective unwinding bound. A loop iterating over such an
/// argument is not fully unwound, so the result does not hold up to the bound after all.
///
/// Called after `add_default_bounds`, so the unwinding bound is resolved by this point.
fn warn_if_bounds_reach_unwind(bounds: AutoharnessBounds, args: &VerificationArgs) {
    // `--unwind` takes precedence over `--default-unwind`, c.f. `resolve_unwind_value`. Automatic
    // harnesses carry no `#[kani::unwind]` attribute, so those two are the whole precedence here.
    let Some(unwind) = args.unwind.or(args.default_unwind) else { return };
    for (option, value) in [
        ("--slice-bound", bounds.slice),
        ("--string-bound", bounds.string),
        ("--bounded-arbitrary-bound", bounds.bounded_arbitrary),
    ] {
        if value >= u64::from(unwind) {
            warning(&format!(
                "{option} is {value}, which is not below the unwinding bound ({unwind}). Loops \
                 iterating over such an argument may not be fully unwound, so verification \
                 results may not hold up to the bound."
            ));
        }
    }
}

/// After generating the automatic harnesses, postprocess metadata and run verification.
fn postprocess_project(
    project: Project,
    session: KaniSession,
    common_autoharness_args: CommonAutoharnessArgs,
) -> Result<()> {
    if !session.args.common_args.quiet {
        print_autoharness_metadata(project.metadata.clone());
    }
    if common_autoharness_args.list {
        let list_metadata = process_metadata(project.metadata.clone());
        return output_list_results(
            list_metadata,
            common_autoharness_args.format,
            session.args.common_args.quiet,
        );
    }
    if session.args.only_codegen {
        Ok(())
    } else if session.args.uses_llbc_backend() {
        codegen_llbc_only(&project, &session)
    } else {
        verify_project(project, session)
    }
}

/// Print automatic harness metadata to the terminal.
fn print_autoharness_metadata(metadata: Vec<KaniMetadata>) {
    let mut chosen_table = PrettyTable::new();
    chosen_table.set_header(vec!["Crate", "Selected Function"]);

    let mut skipped_table = PrettyTable::new();
    skipped_table.set_header(vec!["Crate", "Skipped Function", "Reason for Skipping"]);

    for md in metadata {
        let autoharness_md = md.autoharness_md.unwrap();
        chosen_table.add_rows(
            autoharness_md.chosen.into_iter().map(|func| vec![md.crate_name.clone(), func]),
        );
        skipped_table.add_rows(autoharness_md.skipped.into_iter().filter_map(|(func, reason)| {
            match reason {
                AutoHarnessSkipReason::MissingArbitraryImpl(ref args)
                | AutoHarnessSkipReason::RequiresBoundedArguments(ref args) => Some(vec![
                    md.crate_name.clone(),
                    func,
                    format!(
                        "{reason} {}",
                        args.iter()
                            .map(|(name, typ)| format!("{name}: {typ}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ]),
                AutoHarnessSkipReason::GenericFn(ref detail) => {
                    Some(vec![md.crate_name.clone(), func, format!("{reason}: {detail}")])
                }
                AutoHarnessSkipReason::Comptime
                | AutoHarnessSkipReason::NoBody
                | AutoHarnessSkipReason::UnsupportedVariadic
                | AutoHarnessSkipReason::UserFilter => {
                    Some(vec![md.crate_name.clone(), func, reason.to_string()])
                }
                // We don't report Kani implementations to the user to avoid exposing Kani functions we insert during instrumentation.
                // For those we don't insert during instrumentation that are in this category (manual harnesses or Kani trait implementations),
                // it should be obvious that we wouldn't generate harnesses, so reporting those functions as "skipped" is unlikely to be useful.
                AutoHarnessSkipReason::KaniImpl => None,
            }
        }));
    }

    print_chosen_table(&mut chosen_table);
    print_skipped_table(&mut skipped_table);
}

/// Print the table of functions for which we generated automatic harnesses.
fn print_chosen_table(table: &mut PrettyTable) {
    if table.is_empty() {
        println!(
            "\nSelected Functions: None. Kani did not generate automatic harnesses for any functions in the available crate(s)."
        );
        return;
    }

    println!("\nKani generated automatic harnesses for {} function(s):", table.row_count());
    println!("{table}");
}

/// Print the table of functions for which we did not generate automatic harnesses.
fn print_skipped_table(table: &mut PrettyTable) {
    if table.is_empty() {
        println!(
            "\nSkipped Functions: None. Kani generated automatic harnesses for all functions in the available crate(s)."
        );
        return;
    }

    println!("\nKani did not generate automatic harnesses for {} function(s).", table.row_count());
    println!(
        "If you believe that the provided reason is incorrect and Kani should have generated an automatic harness, please comment on this issue: https://github.com/model-checking/kani/issues/3832"
    );
    println!("{table}");
}

impl KaniSession {
    /// Enable autoharness mode.
    pub fn enable_autoharness(&mut self) {
        self.autoharness_compiler_flags = Some(vec![]);
        self.args.common_args.unstable_features.enable_feature(UnstableFeature::FunctionContracts);
        self.args.common_args.unstable_features.enable_feature(UnstableFeature::LoopContracts);
    }

    /// Add the compiler arguments specific to the `autoharness` subcommand.
    pub fn add_auto_harness_args(
        &mut self,
        included: &[String],
        excluded: &[String],
        bounded_arguments: bool,
        constructor_args: bool,
        check_invariants: bool,
        bounds: AutoharnessBounds,
    ) {
        let mut args = vec![];
        for pattern in included {
            args.push(format!("--autoharness-include-pattern {pattern}"));
        }
        for pattern in excluded {
            args.push(format!("--autoharness-exclude-pattern {pattern}"));
        }
        if bounded_arguments {
            args.push("--autoharness-bounded-arguments".to_string());
            args.push(format!("--autoharness-slice-bound {}", bounds.slice));
            args.push(format!("--autoharness-string-bound {}", bounds.string));
            args.push(format!(
                "--autoharness-bounded-arbitrary-bound {}",
                bounds.bounded_arbitrary
            ));
        }
        if constructor_args {
            args.push("--autoharness-constructor-args".to_string());
        }
        if check_invariants {
            args.push("--autoharness-check-invariants".to_string());
        }
        self.autoharness_compiler_flags = Some(args);
        self.autoharness_bounds = bounds;
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
    pub fn print_autoharness_summary(&self, mut automatic: Vec<&HarnessResult<'_>>) {
        automatic.sort_by(|a, b| a.harness.pretty_name.cmp(&b.harness.pretty_name));
        let (successes, failures): (Vec<_>, Vec<_>) =
            automatic.into_iter().partition(|r| r.result.status == VerificationStatus::Success);

        let succeeding = successes.len();
        let failing = failures.len();
        let total = succeeding + failing;

        println!("\nAutoharness Summary:");

        let mut verified_fns = PrettyTable::new();
        verified_fns.set_header(vec![
            "Crate",
            "Selected Function",
            "Kind of Automatic Harness",
            "Verification Result",
        ]);

        let mut any_bounded = false;
        let mut any_ctor = false;
        let mut any_refcount = false;

        for success in successes {
            any_bounded |= success.harness.is_bounded;
            any_ctor |= success.harness.is_ctor_based;
            any_refcount |= success.harness.is_refcount_bounded;
            verified_fns.add_row(vec![
                success.harness.crate_name.clone(),
                success.harness.pretty_name.clone(),
                harness_kind_label(success.harness),
                success.result.status.to_string(),
            ]);
        }

        for failure in failures {
            any_bounded |= failure.harness.is_bounded;
            any_ctor |= failure.harness.is_ctor_based;
            any_refcount |= failure.harness.is_refcount_bounded;
            verified_fns.add_row(vec![
                failure.harness.crate_name.clone(),
                failure.harness.pretty_name.clone(),
                harness_kind_label(failure.harness),
                failure.result.status.to_string(),
            ]);
        }

        if total > 0 {
            println!("{verified_fns}");
        }

        if any_bounded {
            let bounds = self.autoharness_bounds;
            println!(
                "Note: harnesses marked \"(bounded)\" use bounded nondeterministic values for some arguments (--bounded-arguments):\n\
                 slices up to {} elements, strings up to {} bytes, and BoundedArbitrary \
                 values at bound {}, which each implementation defines for itself.\n\
                 Their verification results only hold up to those bounds, i.e., bugs that require larger input values may be missed.",
                bounds.slice, bounds.string, bounds.bounded_arbitrary
            );
        }
        if any_ctor {
            println!(
                "Note: harnesses marked \"(ctor)\" generate some values through one of a type's own constructors (--constructor-args);\n\
                 their verification results only cover values reachable through that constructor."
            );
        }
        if any_refcount {
            println!("{REFCOUNT_NOTE}");
        }

        if failing > 0 {
            println!(
                "Note that `kani autoharness` sets default --harness-timeout of {AUTOHARNESS_TIMEOUT} and --default-unwind of {LOOP_UNWIND_DEFAULT}."
            );
            println!(
                "If verification failed because of timing out or too low of an unwinding bound, try passing larger values for these arguments (or, if possible, writing a loop contract)."
            );
        }

        if total > 0 {
            println!(
                "Complete - {succeeding} successfully verified functions, {failing} failures, {total} total."
            );
        } else {
            println!("No functions were eligible for automatic verification.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{REFCOUNT_NOTE, harness_kind_label};
    use crate::metadata::tests::mock_proof_harness;

    /// Each caveat adds its own marker; "(refcount)" is independent of "(bounded)", since
    /// `Rc`/`Arc` arguments need no --bounded-arguments (#4752).
    #[test]
    fn harness_kind_label_marks_each_caveat() {
        let mut h = mock_proof_harness("f", None, None, None);
        assert_eq!(harness_kind_label(&h), "#[kani::proof]");
        h.is_refcount_bounded = true;
        assert_eq!(harness_kind_label(&h), "#[kani::proof] (refcount)");
        h.is_bounded = true;
        h.is_ctor_based = true;
        assert_eq!(harness_kind_label(&h), "#[kani::proof] (bounded) (ctor) (refcount)");
    }

    /// The marker fits the summary table's "Kind of Automatic Harness" header, so marking a
    /// proof harness does not widen the column (and shift every row) of existing tables.
    #[test]
    fn refcount_label_fits_kind_column_header() {
        let mut h = mock_proof_harness("f", None, None, None);
        h.is_refcount_bounded = true;
        assert!(harness_kind_label(&h).len() <= "Kind of Automatic Harness".len());
    }

    /// The note names the marker, says it needs no --bounded-arguments, and does not claim
    /// complete coverage.
    #[test]
    fn refcount_note_states_the_caveat() {
        assert!(REFCOUNT_NOTE.contains("\"(refcount)\""));
        assert!(REFCOUNT_NOTE.contains("no --bounded-arguments"));
        assert!(REFCOUNT_NOTE.contains("do not cover every reference count"));
    }
}
