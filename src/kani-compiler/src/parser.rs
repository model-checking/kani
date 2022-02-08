// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, App, AppSettings,
    Arg,
};

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

/// Option name used to dump function pointer restrictions.
pub const RESTRICT_FN_PTRS: &'static str = "restrict-vtable-fn-ptrs";

/// Option name used to override the sysroot.
pub const SYSROOT: &'static str = "sysroot";

/// Option name used to pass extra rustc-options.
pub const RUSTC_OPTIONS: &'static str = "rustc-options";

pub const RUSTC_VERSION: &'static str = "rustc-version";

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
                .possible_values(["error", "warn", "info", "debug", "trace"].as_slice())
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
            Arg::with_name(RESTRICT_FN_PTRS)
                .long("--restrict-vtable-fn-ptrs")
                .help("Restrict the targets of virtual table function pointer calls."),
        )
        .arg(Arg::with_name(SYSROOT).long("--sysroot").help("Override the system root.").long_help(
            "The \"sysroot\" is the location where Kani will look for the Rust \
                distribution.",
        ))
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
}
