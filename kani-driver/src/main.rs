// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use args_toml::join_args;
use call_cbmc::VerificationStatus;
use kani_metadata::HarnessMetadata;
use session::KaniSession;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use util::append_path;

mod args;
mod args_toml;
mod call_cargo;
mod call_cbmc;
mod call_cbmc_viewer;
mod call_display_results;
mod call_goto_cc;
mod call_goto_instrument;
mod call_single_file;
mod call_symtab;
mod cbmc_output_parser;
mod metadata;
mod session;
mod util;

fn main() -> Result<()> {
    match determine_invocation_type(Vec::from_iter(std::env::args_os())) {
        InvocationType::CargoKani(args) => cargokani_main(args),
        InvocationType::Standalone => standalone_main(),
    }
}

fn cargokani_main(input_args: Vec<OsString>) -> Result<()> {
    let input_args = join_args(input_args)?;
    let args = args::CargoKaniArgs::from_iter(input_args);
    args.validate();
    let ctx = session::KaniSession::new(args.common_opts)?;

    let outputs = ctx.cargo_build()?;
    if ctx.args.only_codegen {
        return Ok(());
    }
    let mut goto_objs: Vec<PathBuf> = Vec::new();
    for symtab in &outputs.symtabs {
        goto_objs.push(ctx.symbol_table_to_gotoc(symtab)?);
    }

    let linked_obj = outputs.outdir.join("cbmc-linked.out");
    ctx.link_goto_binary(&goto_objs, &linked_obj)?;
    if let Some(restrictions) = outputs.restrictions {
        ctx.apply_vtable_restrictions(&linked_obj, &restrictions)?;
    }

    let metadata = ctx.collect_kani_metadata(&outputs.metadata)?;
    let harnesses = ctx.determine_targets(&metadata)?;
    let report_base = ctx.args.target_dir.clone().unwrap_or(PathBuf::from("target"));

    let mut failed_harnesses: Vec<&HarnessMetadata> = Vec::new();

    for harness in &harnesses {
        let harness_filename = harness.pretty_name.replace("::", "-");
        let report_dir = report_base.join(format!("report-{}", harness_filename));
        let specialized_obj = outputs.outdir.join(format!("cbmc-for-{}.out", harness_filename));
        ctx.run_goto_instrument(
            &linked_obj,
            &specialized_obj,
            &outputs.symtabs,
            &harness.mangled_name,
        )?;

        let result = ctx.check_harness(&specialized_obj, &report_dir, harness)?;
        if result == VerificationStatus::Failure {
            failed_harnesses.push(harness);
        }
    }

    ctx.print_final_summary(&harnesses, &failed_harnesses)
}

fn standalone_main() -> Result<()> {
    let args = args::StandaloneArgs::from_args();
    args.validate();
    let ctx = session::KaniSession::new(args.common_opts)?;

    let outputs = ctx.compile_single_rust_file(&args.input)?;
    if ctx.args.only_codegen {
        return Ok(());
    }
    let goto_obj = ctx.symbol_table_to_gotoc(&outputs.symtab)?;

    let linked_obj = util::alter_extension(&args.input, "out");
    {
        let mut temps = ctx.temporaries.borrow_mut();
        temps.push(linked_obj.to_owned());
    }
    ctx.link_goto_binary(&[goto_obj], &linked_obj)?;
    if let Some(restriction) = outputs.restrictions {
        ctx.apply_vtable_restrictions(&linked_obj, &restriction)?;
    }

    let metadata = ctx.collect_kani_metadata(&[outputs.metadata])?;
    let harnesses = ctx.determine_targets(&metadata)?;
    let report_base = ctx.args.target_dir.clone().unwrap_or(PathBuf::from("."));

    let mut failed_harnesses: Vec<&HarnessMetadata> = Vec::new();

    for harness in &harnesses {
        let harness_filename = harness.pretty_name.replace("::", "-");
        let report_dir = report_base.join(format!("report-{}", harness_filename));
        let specialized_obj = append_path(&linked_obj, &format!("for-{}", harness_filename));
        {
            let mut temps = ctx.temporaries.borrow_mut();
            temps.push(specialized_obj.to_owned());
        }
        ctx.run_goto_instrument(
            &linked_obj,
            &specialized_obj,
            &[&outputs.symtab],
            &harness.mangled_name,
        )?;

        let result = ctx.check_harness(&specialized_obj, &report_dir, harness)?;
        if result == VerificationStatus::Failure {
            failed_harnesses.push(harness);
        }
    }

    ctx.print_final_summary(&harnesses, &failed_harnesses)
}

impl KaniSession {
    fn check_harness(
        &self,
        binary: &Path,
        report_dir: &Path,
        harness: &HarnessMetadata,
    ) -> Result<VerificationStatus> {
        if !self.args.quiet {
            println!("Checking harness {}...", harness.pretty_name);
        }

        if self.args.visualize {
            self.run_visualize(binary, report_dir, harness)?;
            // Strictly speaking, we're faking success here. This is more "no error"
            Ok(VerificationStatus::Success)
        } else {
            self.run_cbmc(binary, harness)
        }
    }

    fn print_final_summary(
        self,
        harnesses: &[HarnessMetadata],
        failed_harnesses: &[&HarnessMetadata],
    ) -> Result<()> {
        if !self.args.quiet && !self.args.visualize && harnesses.len() > 1 {
            if !failed_harnesses.is_empty() {
                println!("Summary:");
            }
            for harness in failed_harnesses.iter() {
                println!("Verification failed for - {}", harness.pretty_name);
            }

            println!(
                "Complete - {} successfully verified harnesses, {} failures, {} total.",
                harnesses.len() - failed_harnesses.len(),
                failed_harnesses.len(),
                harnesses.len()
            );
        }

        if !failed_harnesses.is_empty() {
            // Failure exit code without additional error message
            drop(self);
            std::process::exit(1);
        }

        Ok(())
    }
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
