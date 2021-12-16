// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(rustc_private, once_cell)]
extern crate rustc_codegen_ssa;
extern crate rustc_driver;

use clap::Arg;
use clap::*;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_driver::{init_env_logger, install_ice_hook, Callbacks, RunCompiler};
use std::lazy::SyncOnceCell;
use std::process;

fn rustc_default_flags() -> Vec<String> {
    let rmc_lib = rmc_lib_path();
    let rmc_deps = rmc_lib.clone() + "/deps";
    let rmc_macros_lib = rmc_macros_path();
    let args = vec![
        "-Z",
        "codegen-backend=gotoc",
        "-C",
        "overflow-checks=on",
        "-Z",
        "trim-diagnostic-paths=no",
        "-Z",
        "human_readable_cgu_names",
        "--cfg=rmc",
        "-Z",
        "crate-attr=feature(register_tool)",
        "-Z",
        "crate-attr=register_tool(rmctool)",
        "-L",
        rmc_lib.as_str(),
        "--extern",
        "rmc",
        "-L",
        rmc_deps.as_str(),
        "--extern",
        "librmc_macros",
        "-L",
        rmc_macros_lib.as_str(),
    ];
    args.iter().map(|s| s.to_string()).collect()
}

fn rmc_lib_path() -> String {
    String::from("lib")
}

fn rmc_macros_path() -> String {
    String::from("lib-macro")
}

fn main() {
    println!("RMC Compiler");
    let args = clap::app_from_crate!()
        .setting(AppSettings::TrailingVarArg) // This allow us to fwd commands to rustc.
        .setting(clap::AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::with_name("rmc-flags")
                .long("--rmc-flags")
                .help("Print the arguments that would be used to call rustc."),
        )
        .arg(
            Arg::with_name("rmc-path")
                .long("--rmc-path")
                .help("Print the arguments that would be used to call rustc."),
        )
        .arg(
            Arg::with_name("rustc-options")
                .help("Arguments to be passed down to rustc.")
                .multiple(true)
                .takes_value(true),
        )
        .get_matches();

    use std::env;

    // Prints each argument on a separate line
    if args.is_present("rmc-flags") {
        println!("{}", rustc_default_flags().join(" "));
    } else {
        let mut rustc_args = vec![String::from("rustc")];
        rustc_args.append(&mut rustc_default_flags());
        rustc_args.append(
            &mut args
                .values_of("rustc-options")
                .unwrap_or(clap::Values::default())
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        );
        compile(rustc_args);
        //compile(env::args().map(|(arg)| { arg.into_string().unwrap() }).collect::<Vec<_>>());
    }
}

struct RmcCallbacks {}

impl Callbacks for RmcCallbacks {}

/// Get the codegen backend based on the name and specified sysroot.
///
/// A name of `None` indicates that the default backend should be used.
#[cfg(rmc)] // TODO: Move rmc crate
pub fn get_codegen_backend() -> Box<dyn CodegenBackend> {
    static LOAD: SyncOnceCell<fn() -> Box<dyn CodegenBackend>> = SyncOnceCell::new();

    let load = LOAD.get_or_init(|| rustc_codegen_rmc::GotocCodegenBackend::new);
    unsafe { load() }
}

fn compile(args: Vec<String>) {
    println!("{:?}", args);
    init_env_logger("RMC_LOG");
    let mut callbacks = RmcCallbacks {};
    install_ice_hook();
    let exit_code = if RunCompiler::new(&args, &mut callbacks)
        //.set_make_codegen_backend(get_codegen_backend)
        .run()
        .is_ok()
    {
        0
    } else {
        1
    };
    process::exit(exit_code)
}
