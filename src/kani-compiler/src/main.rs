// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This is the main entry point to our compiler driver. This code accepts a few options that
//! can be used to configure goto-c compilation as well as all other flags supported by rustc.
//!
//! Like miri, clippy, and other tools developed on the top of rustc, we rely on the
//! rustc_private feature and a specific version of rustc.

#![feature(rustc_private, once_cell)]
extern crate rustc_codegen_ssa;
extern crate rustc_driver;
extern crate rustc_session;

use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, App, AppSettings,
    Arg, ArgMatches,
};
use kani_queries::{QueryDb, UserInput};
use rustc_driver::{init_env_logger, install_ice_hook, Callbacks, RunCompiler};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::rc::Rc;

/// This function generates all rustc configurations required by our goto-c codegen.
fn rustc_gotoc_flags(lib_path: &str) -> Vec<String> {
    let kani_deps = lib_path.clone().to_owned() + "/deps";
    // The option below provides a mechanism by which definitions in the
    // standard library can be overriden. See
    // https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/.E2.9C.94.20Globally.20override.20an.20std.20macro/near/268873354
    // for more details.
    let kani_std_wrapper = format!("noprelude:std={}/libstd.rlib", lib_path);
    let args = vec![
        "-C",
        "overflow-checks=on",
        "-C",
        "panic=abort",
        "-Z",
        "unstable-options",
        "-Z",
        "panic_abort_tests=yes",
        "-Z",
        "trim-diagnostic-paths=no",
        "-Z",
        "human_readable_cgu_names",
        "--cfg=kani",
        "-Z",
        "crate-attr=feature(register_tool)",
        "-Z",
        "crate-attr=register_tool(kanitool)",
        "-L",
        lib_path,
        "--extern",
        "kani",
        "--extern",
        kani_std_wrapper.as_str(),
        "-L",
        kani_deps.as_str(),
    ];
    args.iter().map(|s| s.to_string()).collect()
}

fn parser<'a, 'b>() -> App<'a, 'b> {
    app_from_crate!()
        .setting(AppSettings::TrailingVarArg) // This allow us to fwd commands to rustc.
        .setting(clap::AppSettings::AllowLeadingHyphen)
        .version_short("?")
        .arg(
            Arg::with_name("kani-lib")
                .long("--kani-lib")
                .value_name("FOLDER_PATH")
                .help("Sets the path to locate the kani library.")
                .takes_value(true),
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
                    "The \"sysroot\" is the location where Kani will look for the Rust \
                distribution.",
                ),
        )
        .arg(
            // TODO: Move this to a cargo wrapper. This should return kani version.
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
}

/// Main function. Configure arguments and run the compiler.
fn main() -> Result<(), &'static str> {
    let matches = parser().get_matches();

    // Initialize the logger.
    init_env_logger("KANI_LOG");

    // Generate rustc args.
    let rustc_args = generate_rustc_args(&matches);

    // Configure queries.
    let mut queries = QueryDb::default();
    if let Some(symbol_table_passes) = matches.values_of_os("symbol-table-passes") {
        queries.set_symbol_table_passes(symbol_table_passes.map(convert_arg).collect::<Vec<_>>());
    }
    queries.set_emit_vtable_restrictions(matches.is_present("restrict-vtable-fn-ptrs"));

    // Configure and run compiler.
    let mut callbacks = KaniCallbacks {};
    install_ice_hook();
    let mut compiler = RunCompiler::new(&rustc_args, &mut callbacks);
    if matches.is_present("goto-c") {
        compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| {
            rustc_codegen_kani::GotocCodegenBackend::new(&Rc::new(queries))
        })));
    }
    compiler.run().or(Err("Failed to compile crate."))
}

/// Empty struct since we don't support any callbacks yet.
struct KaniCallbacks {}

/// Use default function implementations.
impl Callbacks for KaniCallbacks {}

/// Generate the arguments to pass to rustc_driver.
fn generate_rustc_args(args: &ArgMatches) -> Vec<String> {
    let mut gotoc_args =
        rustc_gotoc_flags(&args.value_of("kani-lib").unwrap_or(std::env!("KANI_LIB_PATH")));
    let mut rustc_args = vec![String::from("rustc")];
    if args.is_present("goto-c") {
        rustc_args.append(&mut gotoc_args);
    }

    if args.is_present("rustc-version") {
        rustc_args.push(String::from("--version"))
    }

    if let Some(extra_flags) = args.values_of_os("rustc-options") {
        extra_flags.for_each(|arg| rustc_args.push(convert_arg(arg)));
    }
    let sysroot = sysroot_path(args.value_of("sysroot")).expect("[Error] Invalid sysroot. Rebuild Kani or provide the path to rust sysroot using --sysroot option");
    rustc_args.push(String::from("--sysroot"));
    rustc_args.push(convert_arg(sysroot.as_os_str()));
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

/// Convert an argument from OsStr to String.
/// If conversion fails, panic with a custom message.
fn convert_arg(arg: &OsStr) -> String {
    arg.to_str()
        .expect(format!("[Error] Cannot parse argument \"{:?}\".", arg).as_str())
        .to_string()
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

#[cfg(test)]
mod parser_test {
    use super::*;

    #[test]
    fn test_rustc_version() {
        let args = vec!["kani-compiler", "-V"];
        let matches = parser().get_matches_from(args);
        assert!(matches.is_present("rustc-version"));
    }

    #[test]
    fn test_kani_flags() {
        let args = vec!["kani-compiler", "--goto-c", "--kani-lib", "some/path"];
        let matches = parser().get_matches_from(args);
        assert!(matches.is_present("goto-c"));
        assert_eq!(matches.value_of("kani-lib"), Some("some/path"));
    }

    #[cfg(unix)]
    #[test]
    #[should_panic]
    fn test_invalid_arg_fails() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStrExt;

        // The value 0x80 is an invalid character.
        let source = [0x68, 0x65, 0x6C, 0x6C, 0x80];
        let os_str = OsStr::from_bytes(&source[..]);
        assert_eq!(os_str.to_str(), None);

        let matches = parser().get_matches_from(vec![
            OsString::from("--sysroot").as_os_str(),
            OsString::from("any").as_os_str(),
            os_str,
        ]);
        generate_rustc_args(&matches);
    }
}
