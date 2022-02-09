// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::args::KaniArgs;
use crate::util::render_command;
use anyhow::{bail, Context, Result};
use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Contains information about the execution environment and arguments that affect operations
pub struct KaniContext {
    /// The common command-line arguments
    pub args: KaniArgs,

    /// The location we found the 'kani_rustc' command
    pub kani_rustc: PathBuf,
    /// The location we found 'kani_lib.c'
    pub kani_lib_c: PathBuf,
    /// The location we found the Kani C stub .c files
    pub kani_c_stubs: PathBuf,
    /// The location we found kani-link-restrictions binary
    pub kani_link_restrictions: PathBuf,
    /// The location we found 'cbmc_json_parser.py'
    pub cbmc_json_parser_py: PathBuf,

    /// The temporary files we littered that need to be cleaned up at the end of execution
    pub temporaries: RefCell<Vec<PathBuf>>,
}

/// Represents where we detected Kani, with helper methods for using that information to find critical paths
enum InstallType {
    /// We're operating in a a checked out repo that's been built locally
    DevRepo(PathBuf),
    // TODO: Once we have something like an installation method, this should represent where we find the files we installed
    //Installed,
}

impl KaniContext {
    pub fn new(args: KaniArgs) -> Result<Self> {
        let install = InstallType::new()?;

        Ok(KaniContext {
            args,
            kani_rustc: install.kani_rustc()?,
            kani_lib_c: install.kani_lib_c()?,
            kani_c_stubs: install.kani_c_stubs()?,
            kani_link_restrictions: install.kani_link_restrictions()?,
            cbmc_json_parser_py: install.cbmc_json_parser_py()?,
            temporaries: RefCell::new(vec![]),
        })
    }

    pub fn cleanup(self) {
        if !self.args.keep_temps {
            let temporaries = self.temporaries.borrow();

            for file in temporaries.iter() {
                // If it fails, we don't care, skip it
                let _result = std::fs::remove_file(file);
            }
        }
    }

    // The below suite of helper functions for executing Commands are meant to be a common handler
    // for various cmdline flags like 'dry-run' and 'quiet'. These functions are temporary: in the
    // longer run we'll switch to a graph-interpreter style of constructing and executing jobs.
    // (In other words: higher-level data structures, rather than passing around Commands.)
    // (e.g. to support emitting Litani build graphs, or to better parallelize our work)

    // We basically have three different output policies:
    //               No error                  Error                     Notes
    //               Default  Quiet  Verbose   Default  Quiet  Verbose
    // run_terminal  Y        N      Y         Y        N      Y         (inherits terminal)
    // run_suppress  N        N      Y         Y        N      Y         (buffered text only)
    // run_redirect  (not applicable, always to the file)                (only option where error is acceptable)

    /// Run a job, leave it outputting to terminal (unless --quiet), and fail if there's a problem.
    pub fn run_terminal(&self, mut cmd: Command) -> Result<()> {
        if self.args.quiet {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }
        if self.args.verbose || self.args.dry_run {
            println!("{}", render_command(&cmd).to_string_lossy());
            if self.args.dry_run {
                // Short circuit
                return Ok(());
            }
        }
        let result = cmd
            .status()
            .context(format!("Failed to invoke {}", cmd.get_program().to_string_lossy()))?;
        if !result.success() {
            bail!("{} exited with status {}", cmd.get_program().to_string_lossy(), result);
        }
        Ok(())
    }

    /// Run a job, but only output (unless --quiet) if it fails, and fail if there's a problem.
    pub fn run_suppress(&self, mut cmd: Command) -> Result<()> {
        if self.args.quiet || self.args.debug || self.args.verbose || self.args.dry_run {
            return self.run_terminal(cmd);
        }
        let result = cmd
            .output()
            .context(format!("Failed to invoke {}", cmd.get_program().to_string_lossy()))?;
        if !result.status.success() {
            // Don't suppress the output. There doesn't seem to be a way to easily get Command
            // to give one output stream of both out/err with interleaving correct, it seems
            // you'd have to resort to some lower-level interface.
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            handle.write_all(&result.stdout)?;
            handle.write_all(&result.stderr)?;
            bail!("{} exited with status {}", cmd.get_program().to_string_lossy(), result.status);
        }
        Ok(())
    }

    /// Run a job, redirect its output to a file, and allow the caller to decide what to do with failure.
    pub fn run_redirect(&self, mut cmd: Command, stdout: &Path) -> Result<ExitStatus> {
        if self.args.verbose || self.args.dry_run {
            println!("{} > {}", render_command(&cmd).to_string_lossy(), stdout.display());
            if self.args.dry_run {
                // Short circuit. Difficult to mock an ExitStatus :(
                return Ok(<ExitStatus as std::os::unix::prelude::ExitStatusExt>::from_raw(0));
            }
        }
        let output_file = std::fs::File::create(&stdout)?;
        cmd.stdout(output_file);

        return cmd
            .status()
            .context(format!("Failed to invoke {}", cmd.get_program().to_string_lossy()));
    }
}

impl InstallType {
    pub fn new() -> Result<Self> {
        let mut exe = std::env::current_exe()
            .context("cargo-kani was unable to determine where its executable was located")?;
        // Remove the executable name, so we're in the directory we care about
        exe.pop();

        // Case 1: We've checked out the development repo and we're built under `target/`
        if exe.ends_with("target/debug") {
            exe.pop();
            exe.pop();

            Ok(InstallType::DevRepo(exe))
        } else {
            bail!(
                "Unable to determine installation location. {} doesn't look typical",
                exe.display()
            )
        }
    }

    pub fn kani_rustc(&self) -> Result<PathBuf> {
        match self {
            Self::DevRepo(repo) => {
                let mut path = repo.clone();
                path.push("scripts/kani-rustc");
                if path.as_path().exists() {
                    Ok(path)
                } else {
                    bail!("Unable to find kani-rustc. Looked for {}", path.display());
                }
            }
        }
    }

    pub fn kani_lib_c(&self) -> Result<PathBuf> {
        match self {
            Self::DevRepo(repo) => {
                let mut path = repo.clone();
                path.push("library/kani/kani_lib.c");
                if path.as_path().exists() {
                    Ok(path)
                } else {
                    bail!("Unable to find kani_lib.c. Looked for {}", path.display());
                }
            }
        }
    }

    pub fn kani_c_stubs(&self) -> Result<PathBuf> {
        match self {
            Self::DevRepo(repo) => {
                let mut path = repo.clone();
                path.push("library/kani/stubs/C");
                if path.as_path().exists() {
                    Ok(path)
                } else {
                    bail!("Unable to find kani/stubs/C. Looked for {}", path.display());
                }
            }
        }
    }

    pub fn kani_link_restrictions(&self) -> Result<PathBuf> {
        match self {
            Self::DevRepo(repo) => {
                let mut path = repo.clone();
                path.push("target/release/kani-link-restrictions");
                if path.as_path().exists() {
                    Ok(path)
                } else {
                    bail!("Unable to find kani-link-restrictions. Looked for {}", path.display());
                }
            }
        }
    }

    pub fn cbmc_json_parser_py(&self) -> Result<PathBuf> {
        match self {
            Self::DevRepo(repo) => {
                let mut path = repo.clone();
                path.push("scripts/cbmc_json_parser.py");
                if path.as_path().exists() {
                    Ok(path)
                } else {
                    bail!("Unable to find cbmc_json_parser.py. Looked for {}", path.display());
                }
            }
        }
    }
}
