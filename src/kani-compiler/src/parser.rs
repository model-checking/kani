// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, App, AppSettings,
    Arg,
};
use std::env;

/// Option name used to set log level.
pub const LOG_LEVEL: &'static str = "log-level";

/// Option name used to enable goto-c compilation.
pub const GOTO_C: &'static str = "goto-c";

/// Option name used to override Kani library path.
pub const KANI_LIB: &'static str = "kani-lib";

/// Option name used to select symbol table passes.
pub const SYM_TABLE_PASSES: &'static str = "symbol-table-passes";

/// Option name used to set the log output to a json file.
pub const JSON_OUTPUT: &'static str = "json-output";

/// Option name used to force logger to use color output. This doesn't work with --json-output.
pub const COLOR_OUTPUT: &'static str = "color-output";

/// Option name used to dump function pointer restrictions.
pub const RESTRICT_FN_PTRS: &'static str = "restrict-vtable-fn-ptrs";

/// Option name used to enable assertion reachability checks
pub const ASSERTION_REACH_CHECKS: &'static str = "assertion-reach-checks";

/// Option name used to override the sysroot.
pub const SYSROOT: &'static str = "sysroot";

/// Option name used to pass extra rustc-options.
pub const RUSTC_OPTIONS: &'static str = "rustc-options";

pub const RUSTC_VERSION: &'static str = "rustc-version";

/// Environmental variable used to retrieve extra Kani command arguments.
const KANIFLAGS_ENV_VAR: &'static str = "KANIFLAGS";

/// Flag used to indicated that we should retrieve more arguments from `KANIFLAGS' env variable.
const KANI_ARGS_FLAG: &'static str = "--kani-flags";

/// Configure command options for the Kani compiler.
pub fn parser<'a, 'b>() -> App<'a, 'b> {
    app_from_crate!()
        .setting(AppSettings::TrailingVarArg) // This allow us to fwd commands to rustc.
        .setting(clap::AppSettings::AllowLeadingHyphen)
        .version_short("?")
        .arg(
            Arg::with_name(KANI_LIB)
                .long("--kani-lib")
                .value_name("FOLDER_PATH")
                .help("Sets the path to locate the kani library.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(GOTO_C)
                .long("--goto-c")
                .help("Enables compilation to goto-c intermediate representation."),
        )
        .arg(
            Arg::with_name(SYM_TABLE_PASSES)
                .long("--symbol-table-passes")
                .value_name("PASS")
                .help("Transformations to perform to the symbol table after it has been generated.")
                .takes_value(true)
                .use_delimiter(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name(LOG_LEVEL)
                .long("--log-level")
                .takes_value(true)
                .possible_values(&["error", "warn", "info", "debug", "trace"])
                .value_name("LOG_LEVEL")
                .help(
                    "Sets the maximum log level to the value given. Use KANI_LOG for more granular \
            control.",
                ),
        )
        .arg(
            Arg::with_name(JSON_OUTPUT)
                .long("--json-output")
                .help("Print output including logs in json format."),
        )
        .arg(
            Arg::with_name(COLOR_OUTPUT)
                .long("--color-output")
                .help("Print output using colors.")
                .conflicts_with(JSON_OUTPUT),
        )
        .arg(
            Arg::with_name(RESTRICT_FN_PTRS)
                .long("--restrict-vtable-fn-ptrs")
                .help("Restrict the targets of virtual table function pointer calls."),
        )
        .arg(
            Arg::with_name(SYSROOT)
                .long("--sysroot")
                .takes_value(true)
                .help("Override the system root.")
                .long_help(
                    "The \"sysroot\" is the location where Kani will look for the Rust \
                distribution.",
                ),
        )
        .arg(
            // TODO: Move this to a cargo wrapper. This should return kani version.
            Arg::with_name(RUSTC_VERSION)
                .short("V")
                .long("--version")
                .help("Gets underlying rustc version."),
        )
        .arg(
            Arg::with_name(RUSTC_OPTIONS)
                .help("Arguments to be passed down to rustc.")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(ASSERTION_REACH_CHECKS)
                .long("--assertion-reach-checks")
                .help("Check the reachability of every assertion."),
        )
}

/// Retrieves the arguments from the command line and process hack to incorporate CARGO arguments.
///
/// The kani-compiler requires the flags related to the kani libraries to be
/// in front of the ones that control rustc.
///
/// For cargo kani, cargo sometimes adds flags before the custom RUSTFLAGS, hence,
/// we use a special environment variable to set Kani specific flags. These flags
/// should only be enabled if --kani-flags is present.
/// FIXME: Remove this hack once we use cargo build-plan instead.
pub fn command_arguments(args: &Vec<String>) -> Vec<String> {
    assert!(!args.is_empty(), "Arguments should always include executable name");
    let has_kani_flags = args.iter().any(|arg| arg.eq(KANI_ARGS_FLAG));
    if has_kani_flags {
        let mut new_args: Vec<String> = Vec::new();
        new_args.push(args[0].clone());
        let env_flags = env::var(KANIFLAGS_ENV_VAR).unwrap_or(String::new());
        new_args.extend(
            shell_words::split(&env_flags)
                .expect(&format!("Cannot parse {} value '{}'", KANIFLAGS_ENV_VAR, env_flags)),
        );
        new_args.extend(args[1..].iter().filter(|&arg| arg.ne(KANI_ARGS_FLAG)).cloned());
        new_args
    } else {
        args.clone()
    }
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

    #[test]
    fn test_cargo_kani_hack_noop() {
        let args = ["kani-compiler", "some/path"];
        let args = args.map(|v| String::from(v));
        let new_args = command_arguments(&Vec::from(args.clone()));
        assert_eq!(args.as_slice(), new_args.as_slice());
    }

    #[test]
    fn test_cargo_kani_hack_no_args() {
        env::remove_var(KANIFLAGS_ENV_VAR);
        let args = ["kani-compiler", "some/path", "--kani-flags"];
        let args = args.map(|v| String::from(v));
        let new_args = command_arguments(&Vec::from(args.clone()));
        assert_eq!(new_args.len(), 2, "New args should not include --kani-flags");
        assert_eq!(new_args[0], args[0]);
        assert_eq!(new_args[1], args[1]);
    }
}
