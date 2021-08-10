// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Utilities types and procedures to parse and run tests.
//!
//! TODO: The types and procedures in this modules are similar to the ones in
//! `compiletest`. Consider using `Litani` to run the test suites (see
//! [issue #390](https://github.com/model-checking/rmc/issues/390)).

use crate::litani::Litani;
use std::{
    env,
    fmt::{self, Display, Formatter, Write},
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
};

/// Step at which RMC should panic.
#[derive(PartialEq)]
pub enum FailStep {
    /// RMC panics before the codegen step (up to MIR generation). This step
    /// runs the same checks on the test code as `cargo check` including syntax,
    /// type, name resolution, and borrow checks.
    Check,
    /// RMC panics at the codegen step because the test code uses unimplemented
    /// and/or unsupported features.
    Codegen,
    /// RMC panics after the codegen step because of verification failures or
    /// other CBMC errors.
    Verification,
}

impl Display for FailStep {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let str = match self {
            FailStep::Check => "check",
            FailStep::Codegen => "codegen",
            FailStep::Verification => "verify",
        };
        f.write_str(str)
    }
}

/// Data structure representing properties specific to each test.
pub struct TestProps {
    pub path: PathBuf,
    /// How far this test should proceed to start failing.
    pub fail_step: Option<FailStep>,
    /// Extra arguments to pass to `rustc`.
    pub rustc_args: Vec<String>,
    /// Extra arguments to pass to RMC.
    pub rmc_args: Vec<String>,
}

impl TestProps {
    /// Creates a new instance of [`TestProps`] for a test.
    pub fn new(
        path: PathBuf,
        fail_step: Option<FailStep>,
        rustc_args: Vec<String>,
        rmc_args: Vec<String>,
    ) -> Self {
        Self { path, fail_step, rustc_args, rmc_args }
    }
}

impl Display for TestProps {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(fail_step) = &self.fail_step {
            f.write_fmt(format_args!("// rmc-{}-fail\n", fail_step))?;
        }
        if !self.rustc_args.is_empty() {
            f.write_str("// compile-flags:")?;
            for arg in &self.rustc_args {
                f.write_fmt(format_args!(" {}", arg))?;
            }
            f.write_char('\n')?;
        }
        if !self.rmc_args.is_empty() {
            f.write_str("// rmc-flags:")?;
            for arg in &self.rmc_args {
                f.write_fmt(format_args!(" {}", arg))?;
            }
            f.write_char('\n')?;
        }
        Ok(())
    }
}

/// Parses strings of the form `rmc-*-fail` and returns the step at which RMC is
/// expected to panic.
fn try_parse_fail_step(cur_fail_step: Option<FailStep>, line: &str) -> Option<FailStep> {
    let fail_step = if line.contains("rmc-check-fail") {
        Some(FailStep::Check)
    } else if line.contains("rmc-codegen-fail") {
        Some(FailStep::Codegen)
    } else if line.contains("rmc-verify-fail") {
        Some(FailStep::Verification)
    } else {
        None
    };
    match (cur_fail_step.is_some(), fail_step.is_some()) {
        (true, true) => panic!("Error: multiple `rmc-*-fail` headers in a single test."),
        (false, true) => fail_step,
        _ => cur_fail_step,
    }
}

/// Parses strings of the form `<name>-flags: ...` and returns the list of
/// arguments.
fn try_parse_args(cur_args: Vec<String>, name: &str, line: &str) -> Vec<String> {
    let name = format!("{}-flags:", name);
    let mut split = line.split(&name).skip(1);
    let args: Vec<String> = if let Some(rest) = split.next() {
        rest.trim().split_whitespace().map(String::from).collect()
    } else {
        Vec::new()
    };
    match (cur_args.is_empty(), args.is_empty()) {
        (false, false) => panic!("Error: multiple `{}-flags: ...` headers in a single test.", name),
        (true, false) => args,
        _ => cur_args,
    }
}

/// Parses and returns the properties in a test file.
pub fn parse_test_header(path: &Path) -> TestProps {
    let mut fail_step = None;
    let mut rustc_args = Vec::new();
    let mut rmc_args = Vec::new();
    let it = BufReader::new(File::open(path).unwrap());
    for line in it.lines() {
        let line = line.unwrap();
        let line = line.trim_start();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("//") {
            break;
        }
        fail_step = try_parse_fail_step(fail_step, line);
        rustc_args = try_parse_args(rustc_args, "compile", line);
        rmc_args = try_parse_args(rmc_args, "rmc", line);
    }
    TestProps::new(path.to_path_buf(), fail_step, rustc_args, rmc_args)
}

/// Adds RMC and Litani directories to the current `PATH` environment variable.
pub fn add_rmc_and_litani_to_path() {
    let cwd = env::current_dir().unwrap();
    let rmc_dir = cwd.join("scripts");
    let mut litani_dir = cwd.clone();
    litani_dir.extend(["src", "tools", "litani"].iter());
    env::set_var(
        "PATH",
        format!("{}:{}:{}", rmc_dir.display(), litani_dir.display(), env::var("PATH").unwrap()),
    );
}

/// Does RMC catch syntax, type, and borrow errors (if any)?
pub fn add_check_job(litani: &mut Litani, test_props: &TestProps) {
    let exit_status = if test_props.fail_step == Some(FailStep::Check) { 1 } else { 0 };
    let mut rmc_rustc = Command::new("rmc-rustc");
    rmc_rustc.args(&test_props.rustc_args).args(["-Z", "no-codegen"]).arg(&test_props.path);
    // TODO: replace `build` with `check` when Litani adds support for custom
    // stages (see https://github.com/model-checking/rmc/issues/391).
    let mut phony_out = test_props.path.clone();
    phony_out.set_extension("check");
    litani.add_job(
        &rmc_rustc,
        &[&test_props.path],
        &[&phony_out],
        "Is this valid Rust code?",
        test_props.path.to_str().unwrap(),
        "build",
        exit_status,
        5,
    );
}

/// Is RMC expected to codegen all the Rust features in the test?
pub fn add_codegen_job(litani: &mut Litani, test_props: &TestProps) {
    let exit_status = if test_props.fail_step == Some(FailStep::Codegen) { 1 } else { 0 };
    let mut rmc_rustc = Command::new("rmc-rustc");
    rmc_rustc
        .args(&test_props.rustc_args)
        .args(["-Z", "codegen-backend=gotoc", "--cfg=rmc", "--out-dir", "build/tmp"])
        .arg(&test_props.path);
    // TODO: replace `test` with `codegen` when Litani adds support for custom
    // stages (see https://github.com/model-checking/rmc/issues/391).
    let mut phony_in = test_props.path.clone();
    phony_in.set_extension("check");
    let mut phony_out = test_props.path.clone();
    phony_out.set_extension("codegen");
    litani.add_job(
        &rmc_rustc,
        &[&phony_in],
        &[&phony_out],
        "Does RMC support all the Rust features used in it?",
        test_props.path.to_str().unwrap(),
        "test",
        exit_status,
        5,
    );
}

// Does verification pass/fail as it is expected to?
pub fn add_verification_job(litani: &mut Litani, test_props: &TestProps) {
    let exit_status = if test_props.fail_step == Some(FailStep::Verification) { 10 } else { 0 };
    let mut rmc = Command::new("rmc");
    rmc.arg(&test_props.path).args(&test_props.rmc_args);
    if !test_props.rustc_args.is_empty() {
        rmc.env("RUSTFLAGS", test_props.rustc_args.join(" "));
    }
    // TODO: replace `report` with `verification` when Litani adds support for
    // custom stages (see https://github.com/model-checking/rmc/issues/391).
    let mut phony_in = test_props.path.clone();
    phony_in.set_extension("codegen");
    litani.add_job(
        &rmc,
        &[&phony_in],
        &[],
        "Can RMC reason about it?",
        test_props.path.to_str().unwrap(),
        "report",
        exit_status,
        10,
    );
}

/// Creates a new pipeline for the test specified by `path` consisting of 3
/// jobs/steps: `check`, `codegen`, and `verification`.
pub fn add_test_pipeline(litani: &mut Litani, test_props: &TestProps) {
    // The first step ensures that the Rust code in the test compiles (if it is
    // expected to).
    add_check_job(litani, test_props);
    if test_props.fail_step == Some(FailStep::Check) {
        return;
    }
    // The second step ensures that we can codegen the code in the test.
    add_codegen_job(litani, test_props);
    if test_props.fail_step == Some(FailStep::Codegen) {
        return;
    }
    // The final step ensures that CBMC can verify the code in the test.
    // Notice that 10 is the expected error code for verification failure.
    add_verification_job(litani, test_props);
}
