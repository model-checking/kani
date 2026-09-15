// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::str::FromStr;

use crate::args::Timeout;
use crate::args::autoharness_args::{
    CargoAutoharnessArgs, CommonAutoharnessArgs, StandaloneAutoharnessArgs,
};
use crate::args::common::UnstableFeature;
use crate::call_cbmc::VerificationStatus;
use crate::harness_runner::HarnessResult;
use crate::list::collect_metadata::process_metadata;
use crate::list::output::output_list_results;
use crate::project::{Project, standalone_project, std_project};
use crate::session::KaniSession;
use crate::{InvocationType, print_kani_version, project, verify_project};
use anyhow::Result;
use comfy_table::Table as PrettyTable;
use kani_metadata::{AutoHarnessSkipReason, HarnessMetadata, KaniMetadata};

const AUTOHARNESS_TIMEOUT: &str = "60s";
const LOOP_UNWIND_DEFAULT: u32 = 20;

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
    session.add_auto_harness_args(
        &common_autoharness_args.include_pattern,
        &common_autoharness_args.exclude_pattern,
        common_autoharness_args.bounded_arguments,
        common_autoharness_args.constructor_args,
        common_autoharness_args.check_invariants,
    );
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
    if session.args.only_codegen { Ok(()) } else { verify_project(project, session) }
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
                AutoHarnessSkipReason::NoBody | AutoHarnessSkipReason::UserFilter => {
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
        }
        if constructor_args {
            args.push("--autoharness-constructor-args".to_string());
        }
        if check_invariants {
            args.push("--autoharness-check-invariants".to_string());
        }
        self.autoharness_compiler_flags = Some(args);
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

    /// Computes the outcome of automatically-generated harnesses, if autoharness is enabled.
    pub fn autoharness_result<'a, 'pr>(
        &self,
        mut automatic: Vec<&'a HarnessResult<'pr>>,
    ) -> Option<AutoharnessResult<'a, 'pr>> {
        self.autoharness_compiler_flags.as_ref()?;

        automatic.sort_by(|a, b| a.harness.pretty_name.cmp(&b.harness.pretty_name));
        let (successes, failing) =
            automatic.into_iter().partition(|r| r.result.status == VerificationStatus::Success);

        Some(AutoharnessResult { successes, failing })
    }

    /// Prints the results from running the `autoharness` subcommand.
    pub fn print_autoharness_summary(&self, autoharness_result: AutoharnessResult<'_, '_>) {
        let AutoharnessResult { successes, failing } = autoharness_result;

        let succeeding = successes.len();
        let failing_count = failing.len();
        let total = succeeding + failing_count;

        println!("\nAutoharness Summary:");

        let mut verified_fns = PrettyTable::new();
        verified_fns.set_header(vec![
            "Crate",
            "Selected Function",
            "Kind of Automatic Harness",
            "Verification Result",
        ]);

        let harness_kind = |harness: &HarnessMetadata| {
            let mut kind = harness.attributes.kind.to_string();
            if harness.is_bounded {
                kind.push_str(" (bounded)");
            }
            if harness.is_ctor_based {
                kind.push_str(" (ctor)");
            }
            kind
        };
        let mut any_bounded = false;
        let mut any_ctor = false;

        for success in successes {
            any_bounded |= success.harness.is_bounded;
            any_ctor |= success.harness.is_ctor_based;
            verified_fns.add_row(vec![
                success.harness.crate_name.clone(),
                success.harness.pretty_name.clone(),
                harness_kind(success.harness),
                success.result.status.to_string(),
            ]);
        }

        for failure in failing {
            any_bounded |= failure.harness.is_bounded;
            any_ctor |= failure.harness.is_ctor_based;
            verified_fns.add_row(vec![
                failure.harness.crate_name.clone(),
                failure.harness.pretty_name.clone(),
                harness_kind(failure.harness),
                failure.result.status.to_string(),
            ]);
        }

        if total > 0 {
            println!("{verified_fns}");
        }

        if any_bounded {
            println!(
                "Note: harnesses marked \"(bounded)\" use bounded nondeterministic values for some arguments (--bounded-arguments);\n\
                 their verification results only hold up to the bounds, i.e., bugs that require larger input values may be missed."
            );
        }
        if any_ctor {
            println!(
                "Note: harnesses marked \"(ctor)\" generate some values through one of a type's own constructors (--constructor-args);\n\
                 their verification results only cover values reachable through that constructor."
            );
        }

        if failing_count > 0 {
            println!(
                "Note that `kani autoharness` sets default --harness-timeout of {AUTOHARNESS_TIMEOUT} and --default-unwind of {LOOP_UNWIND_DEFAULT}."
            );
            println!(
                "If verification failed because of timing out or too low of an unwinding bound, try passing larger values for these arguments (or, if possible, writing a loop contract)."
            );
        }

        if total > 0 {
            println!(
                "Complete - {succeeding} successfully verified functions, {failing_count} failures, {total} total."
            );
        } else {
            println!("No functions were eligible for automatic verification.");
        }
    }
}

/// The outcome of computing results for automatically-generated harnesses.
pub struct AutoharnessResult<'a, 'pr> {
    successes: Vec<&'a HarnessResult<'pr>>,
    pub(crate) failing: Vec<&'a HarnessResult<'pr>>,
}
