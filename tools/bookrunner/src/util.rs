// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Utilities types and procedures to parse and run tests.
//!
//! TODO: The types and procedures in this modules are similar to the ones in
//! `compiletest`. Consider using `Litani` to run the test suites (see
//! [issue #390](https://github.com/model-checking/kani/issues/390)).

use crate::litani::Litani;
use std::{
    env,
    fmt::{self, Display, Formatter, Write},
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
};

/// Step at which Kani should panic.
#[derive(PartialEq, Eq)]
pub enum FailStep {
    /// Kani panics before the codegen step (up to MIR generation). This step
    /// runs the same checks on the test code as `cargo check` including syntax,
    /// type, name resolution, and borrow checks.
    Check,
    /// Kani panics at the codegen step because the test code uses unimplemented
    /// and/or unsupported features.
    Codegen,
    /// Kani panics after the codegen step because of verification failures or
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
    /// Extra arguments to pass to Kani.
    pub kani_args: Vec<String>,
}

impl TestProps {
    /// Creates a new instance of [`TestProps`] for a test.
    pub fn new(
        path: PathBuf,
        fail_step: Option<FailStep>,
        rustc_args: Vec<String>,
        kani_args: Vec<String>,
    ) -> Self {
        Self { path, fail_step, rustc_args, kani_args }
    }
}

impl Display for TestProps {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(fail_step) = &self.fail_step {
            f.write_fmt(format_args!("// kani-{fail_step}-fail\n"))?;
        }
        if !self.rustc_args.is_empty() {
            f.write_str("// compile-flags:")?;
            for arg in &self.rustc_args {
                f.write_fmt(format_args!(" {arg}"))?;
            }
            f.write_char('\n')?;
        }
        if !self.kani_args.is_empty() {
            f.write_str("// kani-flags:")?;
            for arg in &self.kani_args {
                f.write_fmt(format_args!(" {arg}"))?;
            }
            f.write_char('\n')?;
        }
        Ok(())
    }
}

/// Parses strings of the form `kani-*-fail` and returns the step at which Kani is
/// expected to panic.
fn try_parse_fail_step(cur_fail_step: Option<FailStep>, line: &str) -> Option<FailStep> {
    let fail_step = if line.contains("kani-check-fail") {
        Some(FailStep::Check)
    } else if line.contains("kani-codegen-fail") {
        Some(FailStep::Codegen)
    } else if line.contains("kani-verify-fail") {
        Some(FailStep::Verification)
    } else {
        None
    };
    match (cur_fail_step.is_some(), fail_step.is_some()) {
        (true, true) => panic!("Error: multiple `kani-*-fail` headers in a single test."),
        (false, true) => fail_step,
        _ => cur_fail_step,
    }
}

/// Parses strings of the form `<name>-flags: ...` and returns the list of
/// arguments.
fn try_parse_args(cur_args: Vec<String>, name: &str, line: &str) -> Vec<String> {
    let name = format!("{name}-flags:");
    let mut split = line.split(&name).skip(1);
    let args: Vec<String> = if let Some(rest) = split.next() {
        rest.split_whitespace().map(String::from).collect()
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
    let mut kani_args = Vec::new();
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
        kani_args = try_parse_args(kani_args, "kani", line);
    }
    TestProps::new(path.to_path_buf(), fail_step, rustc_args, kani_args)
}

/// Adds Kani to the current `PATH` environment variable.
pub fn add_kani_to_path() {
    let cwd = env::current_dir().unwrap();
    let kani_bin = cwd.join("target").join("kani").join("bin");
    let kani_scripts = cwd.join("scripts");
    env::set_var(
        "PATH",
        format!("{}:{}:{}", kani_scripts.display(), kani_bin.display(), env::var("PATH").unwrap()),
    );
}

/// Does Kani catch syntax, type, and borrow errors (if any)?
pub fn add_check_job(litani: &mut Litani, test_props: &TestProps) {
    let exit_status = if test_props.fail_step == Some(FailStep::Check) { 1 } else { 0 };
    let mut kani_rustc = Command::new("kani-compiler");
    kani_rustc.args(&test_props.rustc_args).args(["-Z", "no-codegen"]).arg(&test_props.path);

    let mut phony_out = test_props.path.clone();
    phony_out.set_extension("check");
    litani.add_job(
        &kani_rustc,
        &[&test_props.path],
        &[&phony_out],
        "Is this valid Rust code?",
        test_props.path.to_str().unwrap(),
        "check",
        exit_status,
        5,
    );
}

/// Is Kani expected to codegen all the Rust features in the test?
pub fn add_codegen_job(litani: &mut Litani, test_props: &TestProps) {
    let exit_status = if test_props.fail_step == Some(FailStep::Codegen) { 1 } else { 0 };
    let mut kani_rustc = Command::new("kani-compiler");
    kani_rustc.args(&test_props.rustc_args).args(["--out-dir", "build/tmp"]).arg(&test_props.path);

    let mut phony_in = test_props.path.clone();
    phony_in.set_extension("check");
    let mut phony_out = test_props.path.clone();
    phony_out.set_extension("codegen");
    litani.add_job(
        &kani_rustc,
        &[&phony_in],
        &[&phony_out],
        "Does Kani support all the Rust features used in it?",
        test_props.path.to_str().unwrap(),
        "codegen",
        exit_status,
        10,
    );
}

// Does verification pass/fail as it is expected to?
pub fn add_verification_job(litani: &mut Litani, test_props: &TestProps) {
    let exit_status = if test_props.fail_step == Some(FailStep::Verification) { 10 } else { 0 };
    let mut kani = Command::new("kani");
    // Add `--function main` so we can run these without having to amend them to add `#[kani::proof]`.
    // Some of test_props.kani_args will contains `--cbmc-args` so we should always put that last.
    kani.arg(&test_props.path)
        .args(["--enable-unstable", "--function", "main"])
        .args(&test_props.kani_args);
    if !test_props.rustc_args.is_empty() {
        kani.env("RUSTFLAGS", test_props.rustc_args.join(" "));
    }

    let mut phony_in = test_props.path.clone();
    phony_in.set_extension("codegen");
    litani.add_job(
        &kani,
        &[&phony_in],
        &[],
        "Can Kani reason about it?",
        test_props.path.to_str().unwrap(),
        "verification",
        exit_status,
        60,
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
