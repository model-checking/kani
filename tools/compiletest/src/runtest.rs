// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// See GitHub history for details.

// ignore-tidy-filelength

use crate::common::KaniFailStep;
use crate::common::{output_base_dir, output_base_name};
use crate::common::{CargoKani, Expected, Kani, KaniFixme, Stub};
use crate::common::{Config, TestPaths};
use crate::header::TestProps;
use crate::json;
use crate::read2::read2_abbreviated;
use crate::util::logv;
use regex::Regex;

use std::env;
use std::fs::{self, create_dir_all};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::str;

use tracing::*;

#[cfg(not(windows))]
fn disable_error_reporting<F: FnOnce() -> R, R>(f: F) -> R {
    f()
}

/// The name of the environment variable that holds dynamic library locations.
pub fn dylib_env_var() -> &'static str {
    if cfg!(target_os = "macos") { "DYLD_LIBRARY_PATH" } else { "LD_LIBRARY_PATH" }
}

pub fn run(config: Config, testpaths: &TestPaths, revision: Option<&str>) {
    if config.verbose {
        // We're going to be dumping a lot of info. Start on a new line.
        print!("\n\n");
    }
    debug!("running {:?}", testpaths.file.display());
    let props = TestProps::from_file(&testpaths.file, revision, &config);

    let cx = TestCx { config: &config, props: &props, testpaths, revision };
    create_dir_all(&cx.output_base_dir()).unwrap();
    cx.run_revision();
    cx.create_stamp();
}

#[derive(Copy, Clone)]
struct TestCx<'test> {
    config: &'test Config,
    props: &'test TestProps,
    testpaths: &'test TestPaths,
    revision: Option<&'test str>,
}

impl<'test> TestCx<'test> {
    /// Code executed for each revision in turn (or, if there are no
    /// revisions, exactly once, with revision == None).
    fn run_revision(&self) {
        match self.config.mode {
            Kani => self.run_kani_test(),
            KaniFixme => self.run_kani_test(),
            CargoKani => self.run_cargo_kani_test(),
            Expected => self.run_expected_test(),
            Stub => self.run_stub_test(),
        }
    }

    fn compose_and_run(&self, mut command: Command) -> ProcRes {
        let cmdline = {
            let cmdline = format!("{:?}", command);
            logv(self.config, format!("executing {}", cmdline));
            cmdline
        };

        command.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::piped());

        let path =
            env::split_paths(&env::var_os(dylib_env_var()).unwrap_or_default()).collect::<Vec<_>>();

        // Add the new dylib search path var
        let newpath = env::join_paths(&path).unwrap();
        command.env(dylib_env_var(), newpath);

        let child = disable_error_reporting(|| command.spawn())
            .unwrap_or_else(|_| panic!("failed to exec `{:?}`", &command));

        let Output { status, stdout, stderr } =
            read2_abbreviated(child).expect("failed to read output");

        let result = ProcRes {
            status,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            cmdline,
        };

        self.dump_output(&result.stdout, &result.stderr);

        result
    }

    fn dump_output(&self, out: &str, err: &str) {
        let revision = if let Some(r) = self.revision { format!("{}.", r) } else { String::new() };

        self.dump_output_file(out, &format!("{}out", revision));
        self.dump_output_file(err, &format!("{}err", revision));
        self.maybe_dump_to_stdout(out, err);
    }

    fn dump_output_file(&self, out: &str, extension: &str) {
        let outfile = self.make_out_name(extension);
        fs::write(&outfile, out).unwrap();
    }

    /// Creates a filename for output with the given extension.
    /// E.g., `/.../testname.revision.mode/testname.extension`.
    fn make_out_name(&self, extension: &str) -> PathBuf {
        self.output_base_name().with_extension(extension)
    }

    /// The revision, ignored for incremental compilation since it wants all revisions in
    /// the same directory.
    fn safe_revision(&self) -> Option<&str> {
        self.revision
    }

    /// Gets the absolute path to the directory where all output for the given
    /// test/revision should reside.
    /// E.g., `/path/to/build/host-triple/test/ui/relative/testname.revision.mode/`.
    fn output_base_dir(&self) -> PathBuf {
        output_base_dir(self.config, self.testpaths, self.safe_revision())
    }

    /// Gets the absolute path to the base filename used as output for the given
    /// test/revision.
    /// E.g., `/.../relative/testname.revision.mode/testname`.
    fn output_base_name(&self) -> PathBuf {
        output_base_name(self.config, self.testpaths, self.safe_revision())
    }

    fn maybe_dump_to_stdout(&self, out: &str, err: &str) {
        if self.config.verbose {
            println!("------stdout------------------------------");
            println!("{}", out);
            println!("------stderr------------------------------");
            println!("{}", err);
            println!("------------------------------------------");
        }
    }

    fn error(&self, err: &str) {
        match self.revision {
            Some(rev) => println!("\nerror in revision `{}`: {}", rev, err),
            None => println!("\nerror: {}", err),
        }
    }

    fn fatal_proc_rec(&self, err: &str, proc_res: &ProcRes) -> ! {
        self.error(err);
        proc_res.fatal(None, || ());
    }

    /// Adds kani scripts directory to the `PATH` environment variable.
    fn add_kani_dir_to_path(&self, command: &mut Command) {
        // If the PATH environment variable is already defined,
        if let Some((key, val)) = env::vars().find(|(key, _)| key == "PATH") {
            // Add the Kani scripts directory to the PATH.
            command.env(key, format!("{}:{}", self.config.kani_dir_path.to_str().unwrap(), val));
        } else {
            // Otherwise, insert PATH as a new environment variable and set its value to the Kani scripts directory.
            command.env(
                String::from("PATH"),
                String::from(self.config.kani_dir_path.to_str().unwrap()),
            );
        }
    }

    /// Runs `kani-rustc` on the test file specified by `self.testpaths.file`. An
    /// error message is printed to stdout if the check result is not expected.
    fn check(&self) {
        let mut rustc = Command::new("kani-rustc");
        rustc
            .args(["--goto-c"])
            .args(self.props.compile_flags.clone())
            .args(["-Z", "no-codegen"])
            .arg(&self.testpaths.file);
        self.add_kani_dir_to_path(&mut rustc);
        let proc_res = self.compose_and_run(rustc);
        if self.props.kani_panic_step == Some(KaniFailStep::Check) {
            if proc_res.status.success() {
                self.fatal_proc_rec("test failed: expected check failure, got success", &proc_res);
            }
        } else {
            if !proc_res.status.success() {
                self.fatal_proc_rec("test failed: expected check success, got failure", &proc_res);
            }
        }
    }

    /// Runs `kani-rustc` on the test file specified by `self.testpaths.file`. An
    /// error message is printed to stdout if the codegen result is not
    /// expected.
    fn codegen(&self) {
        let mut rustc = Command::new("kani-rustc");
        rustc
            .args(["--goto-c"])
            .args(self.props.compile_flags.clone())
            .args(["--out-dir"])
            .arg(self.output_base_dir())
            .arg(&self.testpaths.file);
        self.add_kani_dir_to_path(&mut rustc);
        let proc_res = self.compose_and_run(rustc);
        if self.props.kani_panic_step == Some(KaniFailStep::Codegen) {
            if proc_res.status.success() {
                self.fatal_proc_rec(
                    "test failed: expected codegen failure, got success",
                    &proc_res,
                );
            }
        } else {
            if !proc_res.status.success() {
                self.fatal_proc_rec(
                    "test failed: expected codegen success, got failure",
                    &proc_res,
                );
            }
        }
    }

    /// Runs Kani on the test file specified by `self.testpaths.file`. An error
    /// message is printed to stdout if the verification result is not expected.
    fn verify(&self) {
        let proc_res = self.run_kani();
        // If the test file contains expected failures in some locations, ensure
        // that verification does indeed fail in those locations
        if proc_res.stdout.contains("EXPECTED FAIL") {
            let lines = TestCx::verify_expect_fail(&proc_res.stdout);
            if !lines.is_empty() {
                self.fatal_proc_rec(
                    &format!("test failed: expected failure in lines {:?}, got success", lines),
                    &proc_res,
                )
            }
        } else {
            // The code above depends too much on the exact string output of
            // Kani. If the output of Kani changes in the future, the check below
            // will (hopefully) force some tests to fail and remind us to
            // update the code above as well.
            if fs::read_to_string(&self.testpaths.file).unwrap().contains("__VERIFIER_expect_fail")
            {
                self.fatal_proc_rec(
                    "found call to `__VERIFIER_expect_fail` with no corresponding \
                 \"EXPECTED FAIL\" in Kani's output",
                    &proc_res,
                )
            }
            // Print an error if the verification result is not expected.
            if self.props.kani_panic_step == Some(KaniFailStep::Verify) {
                if proc_res.status.success() {
                    self.fatal_proc_rec(
                        "test failed: expected verification failure, got success",
                        &proc_res,
                    );
                }
            } else {
                if !proc_res.status.success() {
                    self.fatal_proc_rec(
                        "test failed: expected verification success, got failure",
                        &proc_res,
                    );
                }
            }
        }
    }

    /// Checks, codegens, and verifies the test file specified by
    /// `self.testpaths.file`. An error message is printed to stdout if a result
    /// is not expected.
    fn run_kani_test(&self) {
        self.check();
        if self.props.kani_panic_step == Some(KaniFailStep::Check) {
            return;
        }
        self.codegen();
        if self.props.kani_panic_step == Some(KaniFailStep::Codegen) {
            return;
        }
        self.verify();
    }

    /// If the test file contains expected failures in some locations, ensure
    /// that verification does not succeed in those locations.
    fn verify_expect_fail(str: &str) -> Vec<usize> {
        let re = Regex::new(r"line [0-9]+ EXPECTED FAIL: SUCCESS").unwrap();
        let mut lines = vec![];
        for m in re.find_iter(str) {
            let splits = m.as_str().split_ascii_whitespace();
            let num_str = splits.skip(1).next().unwrap();
            let num = num_str.parse().unwrap();
            lines.push(num);
        }
        lines
    }

    /// Runs cargo-kani on the function specified by the stem of `self.testpaths.file`.
    /// An error message is printed to stdout if verification output does not
    /// contain the expected output in `self.testpaths.file`.
    fn run_cargo_kani_test(&self) {
        // We create our own command for the same reasons listed in `run_kani_test` method.
        let mut cargo = Command::new("cargo");
        // We run `cargo` on the directory where we found the `*.expected` file
        let parent_dir = self.testpaths.file.parent().unwrap();
        // The name of the function to test is the same as the stem of `*.expected` file
        let function_name = self.testpaths.file.file_stem().unwrap().to_str().unwrap();
        cargo
            .arg("kani")
            .args(["--function", function_name])
            .arg("--target-dir")
            .arg(self.output_base_dir().join("target"))
            .current_dir(&parent_dir);
        self.add_kani_dir_to_path(&mut cargo);
        let proc_res = self.compose_and_run(cargo);
        let expected = fs::read_to_string(self.testpaths.file.clone()).unwrap();
        self.verify_output(&proc_res, &expected);
    }

    /// Common method used to run Kani on a single file test.
    fn run_kani(&self) -> ProcRes {
        // Other modes call self.compile_test(...). However, we cannot call it here for two reasons:
        // 1. It calls rustc instead of Kani
        // 2. It may pass some options that do not make sense for Kani
        // So we create our own command to execute Kani and pass it to self.compose_and_run(...) directly.
        let mut kani = Command::new("kani");
        // We cannot pass rustc flags directly to Kani. Instead, we add them
        // to the current environment through the `RUSTFLAGS` environment
        // variable. Kani recognizes the variable and adds those flags to its
        // internal call to rustc.
        if !self.props.compile_flags.is_empty() {
            kani.env("RUSTFLAGS", self.props.compile_flags.join(" "));
        }
        // Pass the test path along with Kani and CBMC flags parsed from comments at the top of the test file.
        kani.arg(&self.testpaths.file)
            .args(&self.props.kani_flags)
            .arg("--cbmc-args")
            .args(&self.props.cbmc_flags);
        self.add_kani_dir_to_path(&mut kani);
        self.compose_and_run(kani)
    }

    /// Runs Kani on the test file specified by `self.testpaths.file`. An error
    /// message is printed to stdout if verification output does not contain
    /// the expected output in `expected` file.
    fn run_expected_test(&self) {
        let proc_res = self.run_kani();
        let expected =
            fs::read_to_string(self.testpaths.file.parent().unwrap().join("expected")).unwrap();
        self.verify_output(&proc_res, &expected);
    }

    /// Runs Kani with stub implementations of various data structures.
    /// Currently, it only runs tests for the Vec module with the (Kani)Vec
    /// abstraction. At a later stage, it should be possible to add command-line
    /// arguments to test specific abstractions and modules.
    fn run_stub_test(&self) {
        let proc_res = self.run_kani();
        if !proc_res.status.success() {
            self.fatal_proc_rec(
                "test failed: expected verification success, got failure",
                &proc_res,
            );
        }
    }

    /// Print an error if the verification output does not contain the expected
    /// lines.
    fn verify_output(&self, proc_res: &ProcRes, expected: &str) {
        // Include the output from stderr here for cases where there are exceptions
        let output = proc_res.stdout.to_string() + &proc_res.stderr;
        if let Some(line) = TestCx::contains_lines(&output, expected.split('\n').collect()) {
            self.fatal_proc_rec(
                &format!("test failed: expected output to contain the line: {:?}", line),
                &proc_res,
            );
        }
    }

    /// Looks for each line in `str`. Returns `None` if all lines are in `str`.
    /// Otherwise, it returns the first line not found in `str`.
    fn contains_lines<'a>(str: &str, lines: Vec<&'a str>) -> Option<String> {
        for line in lines {
            // A "\\" pattern in the expected file is a marker that is used for
            // separating multiple consecutive lines in the output, so
            // substitute those markers with newlines
            let consecutive_lines = line.replace("\\\\", "\n");
            if !str.contains(&consecutive_lines) {
                return Some(consecutive_lines);
            }
        }
        None
    }

    fn create_stamp(&self) {
        let stamp = crate::stamp(&self.config, self.testpaths, self.revision);
        fs::write(&stamp, "we only support one configuration").unwrap();
    }
}

pub struct ProcRes {
    status: ExitStatus,
    stdout: String,
    stderr: String,
    cmdline: String,
}

impl ProcRes {
    pub fn fatal(&self, err: Option<&str>, on_failure: impl FnOnce()) -> ! {
        if let Some(e) = err {
            println!("\nerror: {}", e);
        }
        print!(
            "\
             status: {}\n\
             command: {}\n\
             stdout:\n\
             ------------------------------------------\n\
             {}\n\
             ------------------------------------------\n\
             stderr:\n\
             ------------------------------------------\n\
             {}\n\
             ------------------------------------------\n\
             \n",
            self.status,
            self.cmdline,
            json::extract_rendered(&self.stdout),
            json::extract_rendered(&self.stderr),
        );
        on_failure();
        // Use resume_unwind instead of panic!() to prevent a panic message + backtrace from
        // compiletest, which is unnecessary noise.
        std::panic::resume_unwind(Box::new(()));
    }
}
