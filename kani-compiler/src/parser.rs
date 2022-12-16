// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use clap::value_parser;
use clap::{builder::PossibleValuesParser, command, Arg, ArgAction, ArgMatches, Command};
use kani_queries::ReachabilityType;
use std::env;
use std::ffi::OsString;
use std::str::FromStr;
use strum::VariantNames as _;

/// Option name used to set log level.
pub const LOG_LEVEL: &str = "log-level";

/// Option name used to enable goto-c compilation.
pub const GOTO_C: &str = "goto-c";

/// Option name used to override Kani library path.
pub const KANI_LIB: &str = "kani-lib";

/// Option name used to set the log output to a json file.
pub const JSON_OUTPUT: &str = "json-output";

/// Option name used to force logger to use color output. This doesn't work with --json-output.
pub const COLOR_OUTPUT: &str = "color-output";

/// Option name used to dump function pointer restrictions.
pub const RESTRICT_FN_PTRS: &str = "restrict-vtable-fn-ptrs";

/// Option name used to enable assertion reachability checks.
pub const ASSERTION_REACH_CHECKS: &str = "assertion-reach-checks";

/// Option name used to use json pretty-print for output files.
pub const PRETTY_OUTPUT_FILES: &str = "pretty-json-files";

/// Option used for suppressing global ASM error.
pub const IGNORE_GLOBAL_ASM: &str = "ignore-global-asm";

/// Option name used to override the sysroot.
pub const SYSROOT: &str = "sysroot";

/// Option name used to select which reachability analysis to perform.
pub const REACHABILITY: &str = "reachability";
pub const REACHABILITY_FLAG: &str = "--reachability";

/// Option name used to specify which harness is the target.
pub const HARNESS: &str = "harness";

/// Option name used to enable stubbing.
pub const ENABLE_STUBBING: &str = "enable-stubbing";

/// Option name used to pass extra rustc-options.
pub const RUSTC_OPTIONS: &str = "rustc-options";

pub const RUSTC_VERSION: &str = "rustc-version";

/// Environmental variable used to retrieve extra Kani command arguments.
const KANIFLAGS_ENV_VAR: &str = "KANIFLAGS";

/// Flag used to indicated that we should retrieve more arguments from `KANIFLAGS' env variable.
const KANI_ARGS_FLAG: &str = "--kani-flags";

/// Configure command options for the Kani compiler.
pub fn parser() -> Command {
    let app = command!()
        .disable_version_flag(true)
        .arg(
            Arg::new("kani-compiler-version")
                .short('?')
                .action(ArgAction::Version)
                .help("Gets `kani-compiler` version."),
        )
        .arg(
            Arg::new(KANI_LIB)
                .long(KANI_LIB)
                .value_name("FOLDER_PATH")
                .help("Sets the path to locate the kani library.")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(GOTO_C)
                .long(GOTO_C)
                .help("Enables compilation to goto-c intermediate representation.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(LOG_LEVEL)
                .long(LOG_LEVEL)
                .value_parser(["error", "warn", "info", "debug", "trace"])
                .value_name("LOG_LEVEL")
                .help(
                    "Sets the maximum log level to the value given. Use KANI_LOG for more granular \
            control.",
                )
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(JSON_OUTPUT)
                .long(JSON_OUTPUT)
                .help("Print output including logs in json format.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(COLOR_OUTPUT)
                .long(COLOR_OUTPUT)
                .help("Print output using colors.")
                .conflicts_with(JSON_OUTPUT)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(RESTRICT_FN_PTRS)
                .long(RESTRICT_FN_PTRS)
                .help("Restrict the targets of virtual table function pointer calls.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(SYSROOT)
                .long(SYSROOT)
                .help("Override the system root.")
                .long_help(
                    "The \"sysroot\" is the location where Kani will look for the Rust \
                distribution.",
                )
                .action(ArgAction::Set),
        )
        .arg(
            // TODO: Move this to a cargo wrapper. This should return kani version.
            Arg::new(RUSTC_VERSION)
                .short('V')
                .long("version")
                .help("Gets underlying rustc version.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(RUSTC_OPTIONS)
                .help("Arguments to be passed down to rustc.")
                .trailing_var_arg(true) // This allow us to fwd commands to rustc.
                .allow_hyphen_values(true)
                .value_parser(value_parser!(OsString)) // Allow invalid UTF-8
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(ASSERTION_REACH_CHECKS)
                .long(ASSERTION_REACH_CHECKS)
                .help("Check the reachability of every assertion.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(REACHABILITY)
                .long(REACHABILITY)
                .value_parser(PossibleValuesParser::new(ReachabilityType::VARIANTS))
                .required(false)
                .default_value(ReachabilityType::None.as_ref())
                .help("Selects the type of reachability analysis to perform.")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(PRETTY_OUTPUT_FILES)
                .long(PRETTY_OUTPUT_FILES)
                .help("Output json files in a more human-readable format (with spaces).")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(IGNORE_GLOBAL_ASM)
                .long(IGNORE_GLOBAL_ASM)
                .help("Suppress error due to the existence of global_asm in a crate")
                .action(ArgAction::SetTrue),
        )
        .arg(
            // TODO: Remove this argument once stubbing works for multiple harnesses at a time.
            // <https://github.com/model-checking/kani/issues/1841>
            Arg::new(HARNESS)
                .long(HARNESS)
                .help("Selects the harness to target.")
                .value_name("HARNESS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(ENABLE_STUBBING)
                .long(ENABLE_STUBBING)
                .help("Instruct the compiler to perform stubbing.")
                .requires(HARNESS)
                .action(ArgAction::SetTrue),
        );
    #[cfg(feature = "unsound_experiments")]
    let app = crate::unsound_experiments::arg_parser::add_unsound_experiments_to_parser(app);

    app
}

pub trait KaniCompilerParser {
    fn reachability_type(&self) -> ReachabilityType;
}

impl KaniCompilerParser for ArgMatches {
    fn reachability_type(&self) -> ReachabilityType {
        self.get_one::<String>(REACHABILITY)
            .map_or(ReachabilityType::None, |arg| ReachabilityType::from_str(arg).unwrap())
    }
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
        let mut filter_args = vec![KANI_ARGS_FLAG];
        let mut new_args: Vec<String> = Vec::new();
        new_args.push(args[0].clone());
        // For cargo kani, --reachability is included as a rustc argument.
        let reachability = args.iter().find(|arg| arg.starts_with(REACHABILITY_FLAG));
        if let Some(value) = reachability {
            new_args.push(value.clone());
            filter_args.push(value)
        }
        // Add all the other kani specific arguments are inside $KANIFLAGS.
        let env_flags = env::var(KANIFLAGS_ENV_VAR).unwrap_or_default();
        new_args.extend(
            shell_words::split(&env_flags)
                .expect(&format!("Cannot parse {KANIFLAGS_ENV_VAR} value '{env_flags}'")),
        );
        // Add the leftover arguments for rustc at the end.
        new_args
            .extend(args[1..].iter().filter(|&arg| !filter_args.contains(&arg.as_str())).cloned());
        new_args
    } else {
        args.clone()
    }
}

#[cfg(test)]
mod parser_test {
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    fn test_rustc_version() {
        let args = vec!["kani-compiler", "-V"];
        let matches = parser().get_matches_from(args);
        assert!(matches.get_flag("rustc-version"));
    }

    #[test]
    fn test_kani_flags() {
        let args = vec!["kani-compiler", "--goto-c", "--kani-lib", "some/path"];
        let matches = parser().get_matches_from(args);
        assert!(matches.get_flag("goto-c"));
        assert_eq!(matches.get_one::<String>("kani-lib"), Some(&"some/path".to_string()));
    }

    #[test]
    fn test_stubbing_flags() {
        let args = vec!["kani-compiler", "--enable-stubbing", "--harness", "foo"];
        let matches = parser().get_matches_from(args);
        assert!(matches.get_flag("enable-stubbing"));
        assert_eq!(matches.get_one::<String>("harness"), Some(&"foo".to_string()));

        // `--enable-stubbing` cannot be called without `--harness`
        let args = vec!["kani-compiler", "--enable-stubbing"];
        let err = parser().try_get_matches_from(args).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_cargo_kani_hack_noop() {
        let args = ["kani-compiler", "some/path"];
        let args = args.map(String::from);
        let new_args = command_arguments(&Vec::from(args.clone()));
        assert_eq!(args.as_slice(), new_args.as_slice());
    }

    #[test]
    fn test_cargo_kani_hack_no_args() {
        env::remove_var(KANIFLAGS_ENV_VAR);
        let args = ["kani-compiler", "some/path", "--kani-flags"];
        let args = args.map(String::from);
        let new_args = command_arguments(&Vec::from(args.clone()));
        assert_eq!(new_args.len(), 2, "New args should not include --kani-flags");
        assert_eq!(new_args[0], args[0]);
        assert_eq!(new_args[1], args[1]);
    }
}
