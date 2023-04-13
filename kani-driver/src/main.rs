// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(let_chains)]
#![feature(array_methods)]
use std::ffi::OsString;
use std::process::ExitCode;

use anyhow::Result;

use args::CargoKaniSubcommand;
use args_toml::join_args;

use crate::project::Project;
use crate::session::KaniSession;
use clap::Parser;
use tracing::debug;

mod args;
mod args_toml;
mod assess;
mod call_cargo;
mod call_cbmc;
mod call_cbmc_viewer;
mod call_goto_cc;
mod call_goto_instrument;
mod call_goto_synthesizer;
mod call_single_file;
mod cbmc_output_parser;
mod cbmc_property_renderer;
mod concrete_playback;
mod harness_runner;
mod metadata;
mod project;
mod session;
mod util;

/// The main function for the `kani-driver`.
/// The driver can be invoked via `cargo kani` and `kani` commands, which determines what kind of
/// project should be verified.
fn main() -> ExitCode {
    let result = match determine_invocation_type(Vec::from_iter(std::env::args_os())) {
        InvocationType::CargoKani(args) => cargokani_main(args),
        InvocationType::Standalone => standalone_main(),
    };

    if let Err(error) = result {
        // We are using the debug format for now to print the all the context.
        // We should consider creating a standard for error reporting.
        debug!(?error, "main_failure");
        util::error(&format!("{error:#}"));
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// The main function for the `cargo kani` command.
fn cargokani_main(input_args: Vec<OsString>) -> Result<()> {
    let input_args = join_args(input_args)?;
    let args = args::CargoKaniArgs::parse_from(input_args);
    args.validate();
    let session = session::KaniSession::new(args.common_opts)?;

    if let Some(CargoKaniSubcommand::Assess(args)) = args.command {
        return assess::run_assess(session, args);
    } else if session.args.assess {
        return assess::run_assess(session, assess::AssessArgs::default());
    }

    let project = project::cargo_project(&session, false)?;
    if session.args.only_codegen { Ok(()) } else { verify_project(project, session) }
}

/// The main function for the `kani` command.
fn standalone_main() -> Result<()> {
    let args = args::StandaloneArgs::parse();
    args.validate();
    let session = session::KaniSession::new(args.common_opts)?;

    let project = project::standalone_project(&args.input, &session)?;
    if session.args.only_codegen { Ok(()) } else { verify_project(project, session) }
}

/// Run verification on the given project.
fn verify_project(project: Project, session: KaniSession) -> Result<()> {
    debug!(?project, "verify_project");
    let harnesses = session.determine_targets(&project.get_all_harnesses())?;
    debug!(n = harnesses.len(), ?harnesses, "verify_project");

    // Verification
    let runner = harness_runner::HarnessRunner { sess: &session, project: &project };
    let results = runner.check_all_harnesses(&harnesses)?;

    session.print_final_summary(&results)
}

#[derive(Debug, PartialEq, Eq)]
enum InvocationType {
    CargoKani(Vec<OsString>),
    Standalone,
}

/// Peeks at command line arguments to determine if we're being invoked as 'kani' or 'cargo-kani'
fn determine_invocation_type(mut args: Vec<OsString>) -> InvocationType {
    let exe = util::executable_basename(&args.get(0));

    // Case 1: if 'kani' is our first real argument, then we're being invoked as cargo-kani
    // 'cargo kani ...' will cause cargo to run 'cargo-kani kani ...' preserving argv1
    if Some(&OsString::from("kani")) == args.get(1) {
        // Recreate our command line, but with 'kani' skipped
        args.remove(1);
        InvocationType::CargoKani(args)
    }
    // Case 2: if 'kani' is the name we're invoked as, then we're being invoked standalone
    // Note: we care about argv0 here, NOT std::env::current_exe(), as the later will be resolved
    else if Some("kani".into()) == exe {
        InvocationType::Standalone
    }
    // Case 3: if 'cargo-kani' is the name we're invoked as, then the user is directly invoking
    // 'cargo-kani' instead of 'cargo kani', and we shouldn't alter arguments.
    else if Some("cargo-kani".into()) == exe {
        InvocationType::CargoKani(args)
    }
    // Case 4: default fallback, act like standalone
    else {
        InvocationType::Standalone
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_invocation_type() {
        // conversions to/from OsString are rough, simplify the test code below
        fn x(args: Vec<&str>) -> Vec<OsString> {
            args.iter().map(|x| x.into()).collect()
        }

        // Case 1: 'cargo kani'
        assert_eq!(
            determine_invocation_type(x(vec!["bar", "kani", "foo"])),
            InvocationType::CargoKani(x(vec!["bar", "foo"]))
        );
        // Case 3: 'cargo-kani'
        assert_eq!(
            determine_invocation_type(x(vec!["cargo-kani", "foo"])),
            InvocationType::CargoKani(x(vec!["cargo-kani", "foo"]))
        );
        // Case 2: 'kani'
        assert_eq!(determine_invocation_type(x(vec!["kani", "foo"])), InvocationType::Standalone);
        // default
        assert_eq!(determine_invocation_type(x(vec!["foo"])), InvocationType::Standalone);
        // weird case can be handled
        assert_eq!(determine_invocation_type(x(vec![])), InvocationType::Standalone);
    }
}
