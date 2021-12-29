// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(rustc_private, once_cell)]
extern crate rustc_codegen_ssa;
extern crate rustc_driver;
extern crate rustc_session;

use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
    ArgMatches,
};
use rmc_queries::{QueryDb, UserInput};
use rustc_driver::{init_env_logger, install_ice_hook, Callbacks, RunCompiler};
use std::path::PathBuf;
use std::rc::Rc;

fn rustc_gotoc_flags(lib_path: &str) -> Vec<String> {
    let rmc_deps = lib_path.clone().to_owned() + "/deps";
    let args = vec![
        "-Z",
        "codegen-backend=gotoc",
        "-C",
        "overflow-checks=on",
        "-C",
        "panic=abort",
        "-Z",
        "panic_abort_tests=yes",
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
        lib_path,
        "--extern",
        "rmc",
        "-L",
        rmc_deps.as_str(),
    ];
    args.iter().map(|s| s.to_string()).collect()
}

fn main() -> Result<(), &'static str> {
    let args = app_from_crate!()
        .setting(AppSettings::TrailingVarArg) // This allow us to fwd commands to rustc.
        .setting(clap::AppSettings::AllowLeadingHyphen)
        .version_short("?")
        .arg(
            Arg::with_name("rmc-lib")
                .long("--rmc-lib")
                .value_name("FOLDER_PATH")
                .help("Sets the path to locate the rmc library.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("goto-c")
                .long("--goto-c")
                .help("Enables compilation to goto-c intermediate representation."),
        )
        .arg(
            Arg::with_name("symbol-table-passes")
                .long("--symbol-table-passes")
                .value_name("PASS")
                .help("Transformations to perform to the symbol table after it has been generated.")
                .takes_value(true)
                .use_delimiter(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("restrict-vtable-fn-ptrs")
                .long("--restrict-vtable-fn-ptrs")
                .help("Restrict the targets of virtual table function pointer calls."),
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
            // TODO: Move this to a cargo wrapper. This should return rmc version.
            Arg::with_name("rustc-version")
                .short("V")
                .long("--version")
                .help("Gets underlying rustc version."),
        )
        .arg(
            Arg::with_name("rustc-options")
                .help("Arguments to be passed down to rustc.")
                .multiple(true)
                .takes_value(true),
        )
        .get_matches();

    init_env_logger("RMC_LOG");

    let mut gotoc_args = rustc_gotoc_flags(&args.value_of("rmc-lib").unwrap());
    let rustc_args = generate_rustc_args(&args, &mut gotoc_args);
    let mut queries = QueryDb::default();
    queries.set_symbol_table_passes(args.values_of_lossy("symbol-table-passes").unwrap_or(vec![]));
    queries.set_emit_vtable_restrictions(args.is_present("restrict-vtable-fn-ptrs"));

    let mut callbacks = RmcCallbacks {};
    install_ice_hook();
    let mut compiler = RunCompiler::new(&rustc_args, &mut callbacks);
    if args.is_present("goto-c") {
        compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| {
            rustc_codegen_rmc::GotocCodegenBackend::new(&Rc::new(queries))
        })));
    }
    compiler.run().or(Err("Failed to compile crate."))
}

struct RmcCallbacks {}

impl Callbacks for RmcCallbacks {}

fn generate_rustc_args(args: &ArgMatches, gotoc_args: &mut Vec<String>) -> Vec<String> {
    let mut rustc_args = vec![String::from("rustc")];
    if args.is_present("goto-c") {
        rustc_args.append(gotoc_args);
    }

    if args.is_present("rustc-version") {
        rustc_args.push(String::from("--version"))
    }

    rustc_args.append(
        &mut args
            .values_of("rustc-options")
            .unwrap_or(clap::Values::default())
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    );
    let sysroot = sysroot_path(args.value_of("sysroot")).unwrap();
    rustc_args.push(String::from("--sysroot"));
    rustc_args.push(sysroot.to_string_lossy().to_string());
    tracing::info!(?rustc_args, "Compile");
    rustc_args
}

/// Try to generate the rustup toolchain path.
fn toolchain_path(home: Option<String>, toolchain: Option<String>) -> Option<PathBuf> {
    match (home, toolchain) {
        (Some(home), Some(toolchain)) => {
            Some([home, String::from("toolchains"), toolchain].iter().collect::<PathBuf>())
        }
        _ => None,
    }
}

/// Get the sysroot, following the order bellow:
/// - "--sysroot" command line argument
/// - compile-time environment
///    - $SYSROOT
///    - $RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN
///
/// We currently don't support:
/// - runtime environment
///    - $SYSROOT
///    - $RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN
/// - rustc --sysroot
///
/// since we rely on specific nightly version of rustc which may not be compatible with the workspace rustc.
fn sysroot_path(sysroot_arg: Option<&str>) -> Option<PathBuf> {
    let path = sysroot_arg
        .map(PathBuf::from)
        .or_else(|| std::option_env!("SYSROOT").map(PathBuf::from))
        .or_else(|| {
            let home = std::option_env!("RUSTUP_HOME");
            let toolchain = std::option_env!("RUSTUP_TOOLCHAIN");
            toolchain_path(home.map(String::from), toolchain.map(String::from))
        });
    tracing::debug!(?path, "Sysroot path.");
    path
}
