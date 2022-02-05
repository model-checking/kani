// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::ffi::OsString;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "kani",
    about = "Verify a single Rust file. For more information, see https://github.com/model-checking/rmc"
)]
pub struct StandaloneArgs {
    /// Rust file to verify
    #[structopt(parse(from_os_str))]
    pub input: PathBuf,

    #[structopt(flatten)]
    pub common_opts: KaniArgs,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "cargo-kani",
    about = "Verify a Rust crate. For more information, see https://github.com/model-checking/rmc"
)]
pub struct CargoKaniArgs {
    #[structopt(flatten)]
    pub common_opts: KaniArgs,
}

// Common arguments for invoking Kani. This gets put into KaniContext, whereas
// anything above is "local" to "main"'s control flow.
#[derive(Debug, StructOpt)]
pub struct KaniArgs {
    /// Generate visualizer report to <target-dir>/report/html/index.html
    #[structopt(long)]
    pub visualize: bool,
    /// Keep temporary files generated throughout Kani process
    #[structopt(long)]
    pub keep_temps: bool,

    /// Produce full debug information
    #[structopt(long)]
    pub debug: bool,
    /// Produces no output, just an exit code and requested artifacts; overrides --verbose
    #[structopt(long, short)]
    pub quiet: bool,
    /// Output processing stages and commands, along with minor debug information
    #[structopt(long, short)]
    pub verbose: bool,
    /// Print commands instead of running them
    #[structopt(long)]
    pub dry_run: bool,

    #[structopt(flatten)]
    pub checks: CheckArgs,

    /// Entry point for verification
    #[structopt(long, default_value = "main")]
    pub function: String,
    /// Link external C files referenced by Rust code
    #[structopt(long, parse(from_os_str))]
    pub c_lib: Vec<PathBuf>,
    /// Enable test function verification. Only use this option when the entry point is a test function.
    #[structopt(long)]
    pub tests: bool,
    /// Do not produce error return code on CBMC verification failure
    #[structopt(long)]
    pub allow_cbmc_verification_failure: bool,

    /// Specify the number of bits used for representing object IDs in CBMC
    #[structopt(long, default_value = "16")]
    pub object_bits: u32,
    /// Specify the value used for loop unwinding in CBMC
    #[structopt(long)]
    pub unwind: Option<u32>,
    /// Turn on automatic loop unwinding
    #[structopt(long)]
    pub auto_unwind: bool,
    /// Pass through directly to CBMC; must be the last flag
    #[structopt(long, allow_hyphen_values = true)] // consumes everything
    pub cbmc_args: Vec<OsString>,
    /*
    # Add flags that produce extra artifacts.
    def add_artifact_flags(make_group, add_flag, config):
        default_target = config["default-target"]
        assert default_target is not None, \
            f"Missing item in parser config: \"default-target\".\n" \
            "This is a bug; please report this to https://github.com/model-checking/rmc/issues."

        group = make_group(
            "Artifact flags", "Produce artifacts in addition to a basic Kani report.")
        add_flag(group, "--gen-c", default=False, action=BooleanOptionalAction,
                 help="Generate C file equivalent to inputted program")
        add_flag(group, "--gen-c-runnable", default=False, action=BooleanOptionalAction,
                 help="Generate C file equivalent to inputted program; "
                      "performs additional processing to produce valid C code "
                      "at the cost of some readability")
        add_flag(group, "--target-dir", type=pl.Path, default=default_target, metavar="DIR",
                 help=f"Directory for all generated artifacts; defaults to \"{default_target}\"")

    # Add flags needed for toggling and switching between outputs.
    def add_output_flags(make_group, add_flag, config):

        group = make_group("Output flags", "Toggle between different styles of output")
        add_flag(
            group,
            "--output-format",
            default=OutputStyle.OLD,
            type=OutputStyle,
            action=EnumAction,
            help="Select the format for output")

    # Add flags we don't expect end-users to use.
    def add_developer_flags(make_group, add_flag, config):
        group = make_group(
            "Developer flags", "These are generally meant for use by Kani developers, and are not stable.")
        add_flag(group, "--mangler", default="v0", choices=["v0", "legacy"],
                 help="Change what mangler is used by the Rust compiler")
        add_flag(group, "--use-abs", default=False, action=BooleanOptionalAction,
                 help="Use abstractions for the standard library")
        add_flag(group, "--abs-type", default="std", choices=["std", "kani", "c-ffi", "no-back"],
                 help="Choose abstraction for modules of standard library if available")
        add_flag(group, "--restrict-vtable", default=False, action=BooleanOptionalAction,
                 help="Restrict the targets of virtual table function pointer calls")

        */
}

#[derive(Debug, StructOpt)]
pub struct CheckArgs {
    // Rust argument parsers (/clap) don't have the convenient '--flag' and '--no-flag' boolean pairs, so approximate
    // We're put both here then create helper functions to "intepret"
    /// Turn on all default checks
    #[structopt(long)]
    pub default_checks: bool,
    /// Turn off all default checks
    #[structopt(long)]
    pub no_default_checks: bool,

    /// Turn on default memory safety checks
    #[structopt(long)]
    pub memory_safety_checks: bool,
    /// Turn off default memory safety checks
    #[structopt(long)]
    pub no_memory_safety_checks: bool,

    /// Turn on default overflow checks
    #[structopt(long)]
    pub overflow_checks: bool,
    /// Turn off default overflow checks
    #[structopt(long)]
    pub no_overflow_checks: bool,

    /// Turn on undefined function checks
    #[structopt(long)]
    pub undefined_function_checks: bool,
    /// Turn off undefined function checks
    #[structopt(long)]
    pub no_undefined_function_checks: bool,

    /// Turn on default unwinding checks
    #[structopt(long)]
    pub unwinding_checks: bool,
    /// Turn off default unwinding checks
    #[structopt(long)]
    pub no_unwinding_checks: bool,
}

impl CheckArgs {
    pub fn memory_safety_on(&self) -> bool {
        !self.no_default_checks && !self.no_memory_safety_checks || self.memory_safety_checks
    }
    pub fn overflow_on(&self) -> bool {
        !self.no_default_checks && !self.no_overflow_checks || self.overflow_checks
    }
    pub fn undefined_function_on(&self) -> bool {
        !self.no_default_checks && !self.no_undefined_function_checks
            || self.undefined_function_checks
    }
    pub fn unwinding_on(&self) -> bool {
        !self.no_default_checks && !self.no_unwinding_checks || self.unwinding_checks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_arg_parsing() {
        let a = StandaloneArgs::from_iter(vec!["kani", "file.rs", "--cbmc-args", "--multiple", "args", "--here"]);
        assert_eq!(a.common_opts.cbmc_args, vec!["--multiple", "args", "--here"]);
    }
}
