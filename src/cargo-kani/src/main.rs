// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::PathBuf;
use structopt::StructOpt;

mod args;
mod call_cargo;
mod call_cbmc;
mod call_cbmc_formatter;
mod call_cbmc_viewer;
mod call_goto_cc;
mod call_goto_instrument;
mod call_single_file;
mod call_symtab;
mod context;
mod util;

fn main() -> Result<()> {
    match determine_invocation_type(Vec::from_iter(std::env::args_os())) {
        InvocationType::CargoKani(args) => cargokani_main(args),
        InvocationType::Standalone => standalone_main(),
    }
}

fn cargokani_main(input_args: Vec<OsString>) -> Result<()> {
    let args = args::CargoKaniArgs::from_iter(input_args);
    let ctx = context::KaniContext::new(args.common_opts)?;

    let symtabs = ctx.cargo_build()?;
    let mut goto_objs: Vec<PathBuf> = Vec::new();
    for symtab in &symtabs {
        goto_objs.push(ctx.symbol_table_to_gotoc(symtab)?);
    }
    let linked_obj = {
        let mut outdir = symtabs.get(0).unwrap().parent().unwrap().to_path_buf(); // todo: replace this hack
        outdir.push("cbmc.out");
        outdir
    };

    // here on almost identical to below
    ctx.link_c_lib(&goto_objs, &linked_obj, "main")?;
    ctx.run_goto_instrument(&linked_obj)?;

    if ctx.args.visualize {
        ctx.run_visualize(&linked_obj)?;
    } else {
        ctx.run_cbmc(&linked_obj)?;
    }

    ctx.cleanup();

    Ok(())
}

fn standalone_main() -> Result<()> {
    let args = args::StandaloneArgs::from_args();
    let ctx = context::KaniContext::new(args.common_opts)?;

    let symtab_json = ctx.compile_single_rust_file(&args.input)?;
    let goto_obj = ctx.symbol_table_to_gotoc(&symtab_json)?;
    let linked_obj = util::alter_extension(&args.input, "out");

    // almost identical to above below this line
    ctx.link_c_lib(&[goto_obj], &linked_obj, "main")?;
    ctx.run_goto_instrument(&linked_obj)?;

    if ctx.args.visualize {
        ctx.run_visualize(&linked_obj)?;
    } else {
        ctx.run_cbmc(&linked_obj)?;
    }

    ctx.cleanup();

    Ok(())
}

#[derive(Debug, PartialEq)]
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
