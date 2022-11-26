// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(let_chains)]

use anyhow::Result;
use args::CargoKaniSubcommand;
use args_toml::join_args;
use clap::Parser;
use kani_metadata::artifact::{convert_type, ArtifactType::*};
use std::ffi::OsString;
use std::path::PathBuf;

mod args;
mod args_toml;
mod assess;
mod call_cargo;
mod call_cbmc;
mod call_cbmc_viewer;
mod call_goto_cc;
mod call_goto_instrument;
mod call_single_file;
mod cbmc_output_parser;
mod cbmc_property_renderer;
mod concrete_playback;
mod harness_runner;
mod metadata;
mod session;
mod util;

#[cfg(feature = "unsound_experiments")]
mod unsound_experiments;

fn main() -> Result<()> {
    match determine_invocation_type(Vec::from_iter(std::env::args_os())) {
        InvocationType::CargoKani(args) => cargokani_main(args),
        InvocationType::Standalone => standalone_main(),
    }
}

fn cargokani_main(input_args: Vec<OsString>) -> Result<()> {
    let input_args = join_args(input_args)?;
    let args = args::CargoKaniArgs::parse_from(input_args);
    args.validate();
    let ctx = session::KaniSession::new(args.common_opts)?;

    if matches!(args.command, Some(CargoKaniSubcommand::Assess)) || ctx.args.assess {
        // --assess requires --enable-unstable, but the subcommand needs manual checking
        if !ctx.args.enable_unstable {
            clap::Error::raw(
                clap::error::ErrorKind::MissingRequiredArgument,
                "Assess is unstable and requires 'cargo kani --enable-unstable assess'".to_string(),
            )
            .exit()
        }
        // Run the alternative command instead
        return assess::cargokani_assess_main(ctx);
    }

    let outputs = ctx.cargo_build()?;

    let mut goto_objs: Vec<PathBuf> = Vec::new();
    for symtab in &outputs.symtabs {
        let goto_obj_filename = convert_type(symtab, SymTab, SymTabGoto);
        goto_objs.push(goto_obj_filename);
    }

    if ctx.args.only_codegen {
        return Ok(());
    }

    let linked_obj = outputs.outdir.join("cbmc-linked.out");
    ctx.link_goto_binary(&goto_objs, &linked_obj)?;
    if let Some(restrictions) = outputs.restrictions {
        ctx.apply_vtable_restrictions(&linked_obj, &restrictions)?;
    }

    let metadata = ctx.collect_kani_metadata(&outputs.metadata)?;
    let harnesses = ctx.determine_targets(&metadata)?;
    let report_base = ctx.args.target_dir.clone().unwrap_or(PathBuf::from("target"));

    let runner = harness_runner::HarnessRunner {
        sess: &ctx,
        linked_obj: &linked_obj,
        report_base: &report_base,
        symtabs: &outputs.symtabs,
        retain_specialized_harnesses: true,
    };

    let results = runner.check_all_harnesses(&harnesses)?;

    ctx.print_final_summary(&results)
}

fn standalone_main() -> Result<()> {
    let args = args::StandaloneArgs::parse();
    args.validate();
    let ctx = session::KaniSession::new(args.common_opts)?;

    let outputs = ctx.compile_single_rust_file(&args.input)?;

    let goto_obj = outputs.goto_obj;

    if ctx.args.only_codegen {
        return Ok(());
    }

    let linked_obj = args.input.with_extension(Goto);
    ctx.record_temporary_files(&[&linked_obj]);
    ctx.link_goto_binary(&[goto_obj], &linked_obj)?;
    if let Some(restriction) = outputs.restrictions {
        ctx.apply_vtable_restrictions(&linked_obj, &restriction)?;
    }

    let metadata = ctx.collect_kani_metadata(&[outputs.metadata])?;
    let harnesses = ctx.determine_targets(&metadata)?;
    let report_base = ctx.args.target_dir.clone().unwrap_or(PathBuf::from("."));

    let runner = harness_runner::HarnessRunner {
        sess: &ctx,
        linked_obj: &linked_obj,
        report_base: &report_base,
        symtabs: &[outputs.symtab],
        retain_specialized_harnesses: false,
    };

    let results = runner.check_all_harnesses(&harnesses)?;

    ctx.print_final_summary(&results)
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
