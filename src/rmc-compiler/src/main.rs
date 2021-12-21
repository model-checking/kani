// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(rustc_private, once_cell)]
extern crate rustc_codegen_ssa;
extern crate rustc_driver;
extern crate rustc_session;

use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
};
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_driver::{init_env_logger, install_ice_hook, Callbacks, RunCompiler};
use rustc_session::config;
use std::lazy::SyncOnceCell;
use std::path::PathBuf;
use std::process::Command;

fn rustc_default_flags() -> Vec<String> {
    let rmc_lib = rmc_lib_path();
    let rmc_deps = rmc_lib.clone() + "/deps";
    let rmc_macros_lib = rmc_macros_path();
    let args = vec![
        "-Z",
        "codegen-backend=gotoc",
        "-C",
        "overflow-checks=on",
        "-C",
        "panic=abort",
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

fn main() -> Result<(), &'static str> {
    println!("RMC Compiler");
    std::env::vars().for_each(|(var, value)| println!("{} = {}", var, value));
    let args = app_from_crate!()
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
            Arg::with_name("sysroot")
                .long("--sysroot")
                .help("Override the system root.")
                .long_help(
                    "The \"sysroot\" is the location where RMC will look for the Rust \
                distribution.",
                ),
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
        Ok(())
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
        let sysroot= sysroot_path(args.value_of("sysroot")).unwrap();
        rustc_args.push(String::from("--sysroot"));
        rustc_args.push(sysroot.to_string_lossy().to_string());
        compile(rustc_args)
    }
}

struct RmcCallbacks {}

impl Callbacks for RmcCallbacks {}

/// Get the codegen backend based on the name and specified sysroot.
///
/// A name of `None` indicates that the default backend should be used.
pub fn get_codegen_backend(_config: &config::Options) -> Box<dyn CodegenBackend> {
    static LOAD: SyncOnceCell<fn() -> Box<dyn CodegenBackend>> = SyncOnceCell::new();

    let load = LOAD.get_or_init(|| rustc_codegen_rmc::GotocCodegenBackend::new);
    load()
}

fn compile(args: Vec<String>) -> Result<(), &'static str> {
    println!("{:?}", args.join(" "));
    init_env_logger("RMC_LOG");
    let mut callbacks = RmcCallbacks {};
    install_ice_hook();
    let mut compiler = RunCompiler::new(&args, &mut callbacks);
    compiler.set_make_codegen_backend(Some(Box::new(get_codegen_backend)));
    compiler.run().or(Err("Failed to compile crate."))
}

/// Try to generate the rustup toolchain path.
fn toolchain_path(home: Option<String>, toolchain: Option<String>) -> Option<PathBuf> {
    match(home, toolchain) {
        (Some(home), Some(toolchain)) =>
            Some([home, String::from("toolchains"), toolchain].iter().collect::<PathBuf>()),
        _ => None,
    }
}

/// Get the sysroot, following the order bellow:
/// - "--sysroot" command line argument
/// - runtime environment
///    - $SYSROOT
///    - $RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN
/// - rustc --sysroot
/// - compile-time environment
///    - $SYSROOT
///    - $RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN
/// This is similar to the behavior of other tools such as clippy and miri.
fn sysroot_path(sysroot_arg: Option<&str>) -> Option<PathBuf> {
    let path = sysroot_arg
        .map(PathBuf::from)
        .or_else(|| std::env::var("SYSROOT").ok().map(PathBuf::from))
        .or_else(|| toolchain_path(std::env::var("RUSTUP_HOME").ok(), std::env::var
            ("RUSTUP_TOOLCHAIN").ok()))
        .or_else(|| {
            Command::new("rustc")
                .arg("--print")
                .arg("sysroot")
                .output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
        })
        .or_else(|| std::option_env!("SYSROOT").map(PathBuf::from))
        .or_else(|| {
            let home = std::option_env!("RUSTUP_HOME");
            let toolchain = std::option_env!("RUSTUP_TOOLCHAIN");
            toolchain_path(home.map(String::from), toolchain.map(String::from))
        }
        );
    tracing::debug!(?path, "Sysroot path.");
    path
}
