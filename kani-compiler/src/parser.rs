// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::kani_queries::ReachabilityType;
use clap::{builder::PossibleValuesParser, command, Arg, ArgAction, ArgMatches, Command};
use std::env;
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

/// Option used to write JSON symbol tables instead of GOTO binaries.
pub const WRITE_JSON_SYMTAB: &str = "write-json-symtab";

/// Option name used to select which reachability analysis to perform.
pub const REACHABILITY: &str = "reachability";

/// Option name used to specify which harness is the target.
pub const HARNESS: &str = "harness";

/// Option name used to enable stubbing.
pub const ENABLE_STUBBING: &str = "enable-stubbing";

/// Option name used to define unstable features.
pub const UNSTABLE_FEATURE: &str = "unstable";

/// Configure command options for the Kani compiler.
pub fn parser() -> Command {
    let app = command!()
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
            Arg::new(WRITE_JSON_SYMTAB)
                .long(WRITE_JSON_SYMTAB)
                .help("Instruct the compiler to produce GotoC symbol tables in JSON format instead of GOTO binary format.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            // TODO: Remove this argument once stubbing works for multiple harnesses at a time.
            // <https://github.com/model-checking/kani/issues/1841>
            Arg::new(HARNESS)
                .long(HARNESS)
                .help("Selects the harness to target.")
                .value_name("HARNESS")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(ENABLE_STUBBING)
                .long(ENABLE_STUBBING)
                .help("Instruct the compiler to perform stubbing.")
                .requires(HARNESS)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("check-version")
                .long("check-version")
                .action(ArgAction::Set)
                .help("Pass the kani version to the compiler to ensure cache coherence."),
        )
        .arg(
            Arg::new(UNSTABLE_FEATURE)
                .long(UNSTABLE_FEATURE)
                .help("Enable an unstable feature")
                .value_name("UNSTABLE_FEATURE")
                .action(ArgAction::Append),
        );
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

/// Return whether we should run our flavour of the compiler, and which arguments to pass to rustc.
///
/// We add a `--kani-compiler` argument to run the Kani version of the compiler, which needs to be
/// filtered out before passing the arguments to rustc.
///
/// All other Kani arguments are today located inside `--llvm-args`.
pub fn is_kani_compiler(args: Vec<String>) -> (bool, Vec<String>) {
    assert!(!args.is_empty(), "Arguments should always include executable name");
    const KANI_COMPILER: &str = "--kani-compiler";
    let mut has_kani_compiler = false;
    let new_args = args
        .into_iter()
        .filter(|arg| {
            if arg == KANI_COMPILER {
                has_kani_compiler = true;
                false
            } else {
                true
            }
        })
        .collect();
    (has_kani_compiler, new_args)
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
        let (is_kani, new_args) = is_kani_compiler(Vec::from(args.clone()));
        assert_eq!(args.as_slice(), new_args.as_slice());
        assert!(!is_kani);
    }

    #[test]
    fn test_cargo_kani_hack_no_args() {
        let args = ["kani_compiler", "some/path", "--kani-compiler"];
        let args = args.map(String::from);
        let (is_kani, new_args) = is_kani_compiler(Vec::from(args.clone()));
        assert_eq!(new_args.len(), 2, "New args should not include --kani-compiler");
        assert_eq!(new_args[0], args[0]);
        assert_eq!(new_args[1], args[1]);
        assert!(is_kani);
    }
}
