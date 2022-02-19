// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// See GitHub history for details.

#![crate_name = "compiletest"]
// The `test` crate is the only unstable feature
// allowed here, just to share similar code.
#![feature(test)]

extern crate test;

use crate::common::{output_base_dir, output_relative_path, PanicStrategy};
use crate::common::{Config, Mode, TestPaths};
use crate::util::{logv, top_level};
use getopts::Options;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use test::ColorConfig;
use tracing::*;
use walkdir::WalkDir;

use self::header::make_test_description;

pub mod common;
pub mod header;
mod json;
mod raise_fd_limit;
mod read2;
pub mod runtest;
pub mod util;

fn main() {
    tracing_subscriber::fmt::init();

    let config = parse_config(env::args().collect());

    log_config(&config);
    run_tests(config);
}

pub fn parse_config(args: Vec<String>) -> Config {
    let mut opts = Options::new();
    opts.optopt("", "compile-lib-path", "path to host shared libraries", "PATH")
        .optopt("", "run-lib-path", "path to target shared libraries", "PATH")
        .optopt("", "rustc-path", "path to rustc to use for compiling", "PATH")
        .optopt("", "kani-dir-path", "path to directory where kani is located", "PATH")
        .optopt("", "rustdoc-path", "path to rustdoc to use for compiling", "PATH")
        .optopt("", "rust-demangler-path", "path to rust-demangler to use in tests", "PATH")
        .optopt("", "lldb-python", "path to python to use for doc tests", "PATH")
        .optopt("", "docck-python", "path to python to use for doc tests", "PATH")
        .optopt("", "jsondocck-path", "path to jsondocck to use for doc tests", "PATH")
        .optopt("", "valgrind-path", "path to Valgrind executable for Valgrind tests", "PROGRAM")
        .optflag("", "force-valgrind", "fail if Valgrind tests cannot be run under Valgrind")
        .optopt("", "run-clang-based-tests-with", "path to Clang executable", "PATH")
        .optopt("", "llvm-filecheck", "path to LLVM's FileCheck binary", "DIR")
        .optopt("", "src-base", "directory to scan for test files", "PATH")
        .optopt("", "build-base", "directory to deposit test outputs", "PATH")
        .optopt(
            "",
            "mode",
            "which sort of compile tests to run",
            "run-pass-valgrind | pretty | debug-info | codegen | rustdoc \
            | rustdoc-json | codegen-units | incremental | run-make | ui | js-doc-test | mir-opt | assembly | kani | cargo-kani | expected",
        )
        .optopt(
            "",
            "suite",
            "which suite of compile tests to run. used for nicer error reporting.",
            "SUITE",
        )
        .optopt(
            "",
            "pass",
            "force {check,build,run}-pass tests to this mode.",
            "check | build | run",
        )
        .optopt("", "run", "whether to execute run-* tests", "auto | always | never")
        .optflag("", "ignored", "run tests marked as ignored")
        .optflag("", "exact", "filters match exactly")
        .optopt(
            "",
            "runtool",
            "supervisor program to run tests under \
             (eg. emulator, valgrind)",
            "PROGRAM",
        )
        .optmulti("", "host-rustcflags", "flags to pass to rustc for host", "FLAGS")
        .optmulti("", "target-rustcflags", "flags to pass to rustc for target", "FLAGS")
        .optopt("", "target-panic", "what panic strategy the target supports", "unwind | abort")
        .optflag("", "verbose", "run tests verbosely, showing all output")
        .optflag(
            "",
            "bless",
            "overwrite stderr/stdout files instead of complaining about a mismatch",
        )
        .optflag("", "quiet", "print one character per test instead of one line")
        .optopt("", "color", "coloring: auto, always, never", "WHEN")
        .optopt("", "logfile", "file to log test execution to", "FILE")
        .optopt("", "target", "the target to build for", "TARGET")
        .optopt("", "host", "the host to build for", "HOST")
        .optopt("", "cdb", "path to CDB to use for CDB debuginfo tests", "PATH")
        .optopt("", "gdb", "path to GDB to use for GDB debuginfo tests", "PATH")
        .optopt("", "lldb-version", "the version of LLDB used", "VERSION STRING")
        .optopt("", "llvm-version", "the version of LLVM used", "VERSION STRING")
        .optflag("", "system-llvm", "is LLVM the system LLVM")
        .optopt("", "android-cross-path", "Android NDK standalone path", "PATH")
        .optopt("", "adb-path", "path to the android debugger", "PATH")
        .optopt("", "adb-test-dir", "path to tests for the android debugger", "PATH")
        .optopt("", "lldb-python-dir", "directory containing LLDB's python module", "PATH")
        .optopt("", "cc", "path to a C compiler", "PATH")
        .optopt("", "cxx", "path to a C++ compiler", "PATH")
        .optopt("", "cflags", "flags for the C compiler", "FLAGS")
        .optopt("", "ar", "path to an archiver", "PATH")
        .optopt("", "linker", "path to a linker", "PATH")
        .optopt("", "llvm-components", "list of LLVM components built in", "LIST")
        .optopt("", "llvm-bin-dir", "Path to LLVM's `bin` directory", "PATH")
        .optopt("", "nodejs", "the name of nodejs", "PATH")
        .optopt("", "npm", "the name of npm", "PATH")
        .optopt("", "remote-test-client", "path to the remote test client", "PATH")
        .optopt(
            "",
            "compare-mode",
            "mode describing what file the actual ui output will be compared to",
            "COMPARE MODE",
        )
        .optflag(
            "",
            "rustfix-coverage",
            "enable this to generate a Rustfix coverage file, which is saved in \
                `./<build_base>/rustfix_missing_coverage.txt`",
        )
        .optflag("", "force-rerun", "rerun tests even if the inputs are unchanged")
        .optflag("h", "help", "show this message")
        .optopt("", "edition", "default Rust edition", "EDITION");

    let (argv0, args_) = args.split_first().unwrap();
    if args.len() == 1 || args[1] == "-h" || args[1] == "--help" {
        let message = format!("Usage: {} [OPTIONS] [TESTNAME...]", argv0);
        println!("{}", opts.usage(&message));
        println!();
        panic!()
    }

    let matches = &match opts.parse(args_) {
        Ok(m) => m,
        Err(f) => panic!("{:?}", f),
    };

    if matches.opt_present("h") || matches.opt_present("help") {
        let message = format!("Usage: {} [OPTIONS]  [TESTNAME...]", argv0);
        println!("{}", opts.usage(&message));
        println!();
        panic!()
    }

    fn opt_path(m: &getopts::Matches, nm: &str, default: &[&str]) -> PathBuf {
        match m.opt_str(nm) {
            Some(s) => PathBuf::from(&s),
            None => {
                let mut root_folder = top_level().expect(
                    format!("Cannot find root directory. Please provide --{} option.", nm).as_str(),
                );
                default.into_iter().for_each(|f| root_folder.push(f));
                root_folder
            }
        }
    }

    let target = opt_str2(matches.opt_str("target"));
    let color = match matches.opt_str("color").as_deref() {
        Some("auto") | None => ColorConfig::AutoColor,
        Some("always") => ColorConfig::AlwaysColor,
        Some("never") => ColorConfig::NeverColor,
        Some(x) => panic!("argument for --color must be auto, always, or never, but found `{}`", x),
    };

    let suite = matches.opt_str("suite").unwrap();
    let src_base = opt_path(matches, "src-base", &["tests", suite.as_str()]);
    let run_ignored = matches.opt_present("ignored");
    let mode = matches.opt_str("mode").unwrap().parse().expect("invalid mode");

    Config {
        kani_dir_path: opt_path(matches, "kani-dir-path", &["scripts"]),
        src_base,
        build_base: opt_path(matches, "build-base", &["build", "tests", suite.as_str()]),
        mode,
        suite,
        run_ignored,
        filters: matches.free.clone(),
        filter_exact: matches.opt_present("exact"),
        logfile: matches.opt_str("logfile").map(|s| PathBuf::from(&s)),
        host_rustcflags: Some(matches.opt_strs("host-rustcflags").join(" ")),
        target_rustcflags: Some(matches.opt_strs("target-rustcflags").join(" ")),
        target_panic: match matches.opt_str("target-panic").as_deref() {
            Some("unwind") | None => PanicStrategy::Unwind,
            Some("abort") => PanicStrategy::Abort,
            _ => panic!("unknown `--target-panic` option `{}` given", mode),
        },
        target,
        host: opt_str2(matches.opt_str("host")),
        verbose: matches.opt_present("verbose"),
        quiet: matches.opt_present("quiet"),
        color,
        edition: matches.opt_str("edition"),

        force_rerun: matches.opt_present("force-rerun"),
    }
}

pub fn log_config(config: &Config) {
    let c = config;
    logv(c, "configuration:".to_string());
    logv(c, format!("src_base: {:?}", config.src_base.display()));
    logv(c, format!("build_base: {:?}", config.build_base.display()));
    logv(c, format!("mode: {}", config.mode));
    logv(c, format!("run_ignored: {}", config.run_ignored));
    logv(c, format!("filters: {:?}", config.filters));
    logv(c, format!("filter_exact: {}", config.filter_exact));
    logv(c, format!("host-rustcflags: {}", opt_str(&config.host_rustcflags)));
    logv(c, format!("target-rustcflags: {}", opt_str(&config.target_rustcflags)));
    logv(c, format!("target: {}", config.target));
    logv(c, format!("host: {}", config.host));
    logv(c, format!("verbose: {}", config.verbose));
    logv(c, format!("quiet: {}", config.quiet));
    logv(c, "\n".to_string());
}

pub fn opt_str(maybestr: &Option<String>) -> &str {
    match *maybestr {
        None => "(none)",
        Some(ref s) => s,
    }
}

pub fn opt_str2(maybestr: Option<String>) -> String {
    match maybestr {
        None => "(none)".to_owned(),
        Some(s) => s,
    }
}

pub fn run_tests(config: Config) {
    // sadly osx needs some file descriptor limits raised for running tests in
    // parallel (especially when we have lots and lots of child processes).
    // For context, see #8904
    unsafe {
        raise_fd_limit::raise_fd_limit();
    }
    // Prevent issue #21352 UAC blocking .exe containing 'patch' etc. on Windows
    // If #11207 is resolved (adding manifest to .exe) this becomes unnecessary
    env::set_var("__COMPAT_LAYER", "RunAsInvoker");

    // Let tests know which target they're running as
    env::set_var("TARGET", &config.target);

    let opts = test_opts(&config);

    let mut configs = Vec::new();
    configs.push(config.clone());

    let mut tests = Vec::new();
    for c in &configs {
        make_tests(c, &mut tests);
    }

    let res = test::run_tests_console(&opts, tests);
    match res {
        Ok(true) => {}
        Ok(false) => {
            // We want to report that the tests failed, but we also want to give
            // some indication of just what tests we were running. Especially on
            // CI, where there can be cross-compiled tests for a lot of
            // architectures, without this critical information it can be quite
            // easy to miss which tests failed, and as such fail to reproduce
            // the failure locally.

            eprintln!(
                "Some tests failed in compiletest suite={} mode={} host={} target={}",
                config.suite, config.mode, config.host, config.target
            );

            std::process::exit(1);
        }
        Err(e) => {
            // We don't know if tests passed or not, but if there was an error
            // during testing we don't want to just succeed (we may not have
            // tested something), so fail.
            //
            // This should realistically "never" happen, so don't try to make
            // this a pretty error message.
            panic!("I/O failure during tests: {:?}", e);
        }
    }
}

pub fn test_opts(config: &Config) -> test::TestOpts {
    test::TestOpts {
        exclude_should_panic: false,
        filters: config.filters.clone(),
        filter_exact: config.filter_exact,
        run_ignored: if config.run_ignored { test::RunIgnored::Yes } else { test::RunIgnored::No },
        format: if config.quiet { test::OutputFormat::Terse } else { test::OutputFormat::Pretty },
        logfile: config.logfile.clone(),
        run_tests: true,
        bench_benchmarks: true,
        nocapture: match env::var("RUST_TEST_NOCAPTURE") {
            Ok(val) => &val != "0",
            Err(_) => false,
        },
        color: config.color,
        shuffle: false,
        shuffle_seed: None,
        test_threads: None,
        skip: vec![],
        list: false,
        options: test::Options::new(),
        time_options: None,
        force_run_in_process: false,
    }
}

pub fn make_tests(config: &Config, tests: &mut Vec<test::TestDescAndFn>) {
    debug!("making tests from {:?}", config.src_base.display());
    let inputs = common_inputs_stamp();
    collect_tests_from_dir(config, &config.src_base, &PathBuf::new(), &inputs, tests)
        .unwrap_or_else(|_| panic!("Could not read tests from {}", config.src_base.display()));
}

/// Returns a stamp constructed from input files common to all test cases.
fn common_inputs_stamp() -> Stamp {
    let rust_src_dir = top_level().expect("Could not find Rust source root");
    let kani_bin_path = &rust_src_dir.join("target/debug/kani-compiler");

    // Create stamp based on the `kani-compiler` binary
    let mut stamp = Stamp::from_path(&kani_bin_path);

    // Add source, library and script directories
    stamp.add_dir(&rust_src_dir.join("src/"));
    stamp.add_dir(&rust_src_dir.join("library/"));
    stamp.add_dir(&rust_src_dir.join("scripts/"));

    // Add relevant tools directories
    stamp.add_dir(&rust_src_dir.join("tools/compiletest/"));
    stamp.add_dir(&rust_src_dir.join("tools/kani-link-restrictions/"));

    stamp
}

fn collect_tests_from_dir(
    config: &Config,
    dir: &Path,
    relative_dir_path: &Path,
    inputs: &Stamp,
    tests: &mut Vec<test::TestDescAndFn>,
) -> io::Result<()> {
    // If we find a test foo/bar.rs, we have to build the
    // output directory `$build/foo` so we can write
    // `$build/foo/bar` into it. We do this *now* in this
    // sequential loop because otherwise, if we do it in the
    // tests themselves, they race for the privilege of
    // creating the directories and sometimes fail randomly.
    let build_dir = output_relative_path(config, relative_dir_path);
    fs::create_dir_all(&build_dir).unwrap();

    // If we find a `Cargo.toml` file in the current directory and we're in
    // Cargo-kani mode, we should look for `*.expected` files and create an
    // output directory corresponding to each to avoid race conditions during
    // the testing phase. We immediately return after adding the tests to avoid
    // treating `*.rs` files as tests.
    if config.mode == Mode::CargoKani && dir.join("Cargo.toml").exists() {
        for file in fs::read_dir(dir)? {
            let file_path = file?.path();
            if file_path.to_str().unwrap().ends_with(".expected") {
                fs::create_dir_all(&build_dir.join(file_path.file_stem().unwrap())).unwrap();
                let paths =
                    TestPaths { file: file_path, relative_dir: relative_dir_path.to_path_buf() };
                tests.extend(make_test(config, &paths, inputs));
            }
        }
        return Ok(());
    }

    // Add each `.rs` file as a test, and recurse further on any
    // subdirectories we find, except for `aux` directories.
    for file in fs::read_dir(dir)? {
        let file = file?;
        let file_path = file.path();
        let file_name = file.file_name();
        if is_test(&file_name) {
            debug!("found test file: {:?}", file_path.display());
            let paths =
                TestPaths { file: file_path, relative_dir: relative_dir_path.to_path_buf() };
            tests.extend(make_test(config, &paths, inputs))
        } else if file_path.is_dir() {
            let relative_file_path = relative_dir_path.join(file.file_name());
            if &file_name != "auxiliary" {
                debug!("found directory: {:?}", file_path.display());
                collect_tests_from_dir(config, &file_path, &relative_file_path, inputs, tests)?;
            }
        } else {
            debug!("found other file/directory: {:?}", file_path.display());
        }
    }
    Ok(())
}

/// Returns true if `file_name` looks like a proper test file name.
pub fn is_test(file_name: &OsString) -> bool {
    let file_name = file_name.to_str().unwrap();

    if !file_name.ends_with(".rs") {
        return false;
    }

    // `.`, `#`, and `~` are common temp-file prefixes.
    let invalid_prefixes = &[".", "#", "~"];
    !invalid_prefixes.iter().any(|p| file_name.starts_with(p))
}

fn make_test(config: &Config, testpaths: &TestPaths, inputs: &Stamp) -> Vec<test::TestDescAndFn> {
    let test_path = PathBuf::from(&testpaths.file);
    let revisions = vec![None];

    revisions
        .into_iter()
        .map(|revision: Option<&String>| {
            let src_file =
                std::fs::File::open(&test_path).expect("open test file to parse ignores");
            let cfg = revision.map(|v| &**v);
            let test_name = crate::make_test_name(config, testpaths, revision);
            let mut desc = make_test_description(config, test_name, &test_path, src_file, cfg);
            // Ignore tests that already run and are up to date with respect to inputs.
            if !config.force_rerun {
                desc.ignore |=
                    is_up_to_date(config, testpaths, revision.map(|s| s.as_str()), inputs);
            }
            test::TestDescAndFn { desc, testfn: make_test_closure(config, testpaths, revision) }
        })
        .collect()
}

fn stamp(config: &Config, testpaths: &TestPaths, revision: Option<&str>) -> PathBuf {
    output_base_dir(config, testpaths, revision).join("stamp")
}

fn is_up_to_date(
    config: &Config,
    testpaths: &TestPaths,
    revision: Option<&str>,
    inputs: &Stamp,
) -> bool {
    let stamp_name = stamp(config, testpaths, revision);
    // Check timestamps.
    let inputs = inputs.clone();

    inputs < Stamp::from_path(&stamp_name)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Stamp {
    time: SystemTime,
}

impl Stamp {
    fn from_path(path: &Path) -> Self {
        let mut stamp = Stamp { time: SystemTime::UNIX_EPOCH };
        stamp.add_path(path);
        stamp
    }

    fn add_path(&mut self, path: &Path) {
        let modified = fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        self.time = self.time.max(modified);
    }

    fn add_dir(&mut self, path: &Path) {
        for entry in WalkDir::new(path) {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                self.time = self.time.max(modified);
            }
        }
    }
}

fn make_test_name(
    config: &Config,
    testpaths: &TestPaths,
    revision: Option<&String>,
) -> test::TestName {
    // Convert a complete path to something like
    //
    //    ui/foo/bar/baz.rs
    let path = PathBuf::from(config.src_base.file_name().unwrap())
        .join(&testpaths.relative_dir)
        .join(&testpaths.file.file_name().unwrap());

    test::DynTestName(format!(
        "[{}] {}{}",
        config.mode,
        path.display(),
        revision.map_or("".to_string(), |rev| format!("#{}", rev))
    ))
}

fn make_test_closure(
    config: &Config,
    testpaths: &TestPaths,
    revision: Option<&String>,
) -> test::TestFn {
    let config = config.clone();
    let testpaths = testpaths.clone();
    let revision = revision.cloned();
    test::DynTestFn(Box::new(move || runtest::run(config, &testpaths, revision.as_deref())))
}
