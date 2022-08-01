// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::args::KaniArgs;
use crate::util::render_command;
use anyhow::{bail, Context, Result};
use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};

/// Contains information about the execution environment and arguments that affect operations
pub struct KaniSession {
    /// The common command-line arguments
    pub args: KaniArgs,

    /// The location we found the 'kani_rustc' command
    pub kani_compiler: PathBuf,
    /// The location we found 'kani_lib.c'
    pub kani_lib_c: PathBuf,
    /// The location we found the Kani C stub .c files
    pub kani_c_stubs: PathBuf,
    /// The location we found 'cbmc_json_parser.py'
    pub cbmc_json_parser_py: PathBuf,

    /// The location we found our pre-built libraries
    pub kani_rlib: Option<PathBuf>,

    /// The temporary files we littered that need to be cleaned up at the end of execution
    pub temporaries: RefCell<Vec<PathBuf>>,
}

/// Represents where we detected Kani, with helper methods for using that information to find critical paths
enum InstallType {
    /// We're operating in a a checked out repo that's been built locally.
    /// The path here is to the root of the repo.
    DevRepo(PathBuf),
    /// We're operating from a release bundle (made with `make-kani-release`).
    /// The path here to where this release bundle has been unpacked.
    Release(PathBuf),
}

impl KaniSession {
    pub fn new(args: KaniArgs) -> Result<Self> {
        let install = InstallType::new()?;

        Ok(KaniSession {
            args,
            kani_compiler: install.kani_compiler()?,
            kani_lib_c: install.kani_lib_c()?,
            kani_c_stubs: install.kani_c_stubs()?,
            cbmc_json_parser_py: install.cbmc_json_parser_py()?,
            kani_rlib: install.kani_rlib()?,
            temporaries: RefCell::new(vec![]),
        })
    }
}

impl Drop for KaniSession {
    fn drop(&mut self) {
        if !self.args.keep_temps && !self.args.dry_run {
            let temporaries = self.temporaries.borrow();

            for file in temporaries.iter() {
                // If it fails, we don't care, skip it
                let _result = std::fs::remove_file(file);
            }
        }
    }
}

impl KaniSession {
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

        cmd.status().context(format!("Failed to invoke {}", cmd.get_program().to_string_lossy()))
    }

    /// Run a job and pipe its output to this process.
    /// Returns an error if the process could not be spawned
    pub fn run_piped(&self, mut cmd: Command) -> Result<Option<Child>> {
        if self.args.verbose || self.args.dry_run {
            println!("{}", render_command(&cmd).to_string_lossy());
            if self.args.dry_run {
                return Ok(None);
            }
        }
        // Run the process as a child process
        let process = cmd.stdout(Stdio::piped()).spawn();

        // Render the command if the process could not be spawned
        if process.is_err() {
            bail!("Could not spawn process `{}`", render_command(&cmd).to_string_lossy());
        }
        // Return the child process handle
        Ok(Some(process.unwrap()))
    }
}

/// Return the path for the folder where the current executable is located.
fn bin_folder() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("Cannot determine current executable location")?;
    let dir = exe.parent().context("Executable isn't in a directory")?.to_owned();
    Ok(dir)
}

impl InstallType {
    pub fn new() -> Result<Self> {
        // Case 1: We've checked out the development repo and we're built under `target/`
        let mut path = bin_folder()?;
        if path.ends_with("target/debug") || path.ends_with("target/release") {
            path.pop();
            path.pop();

            Ok(InstallType::DevRepo(path))
        } else if path.ends_with("bin") {
            path.pop();

            Ok(InstallType::Release(path))
        } else {
            bail!(
                "Unable to determine installation location. {} doesn't look typical",
                path.display()
            )
        }
    }

    pub fn kani_compiler(&self) -> Result<PathBuf> {
        match self {
            Self::DevRepo(_) => {
                // Use bin_folder to hide debug/release differences.
                let path = bin_folder()?.join("kani-compiler");
                expect_path(path)
            }
            Self::Release(release) => {
                let path = release.join("bin/kani-compiler");
                expect_path(path)
            }
        }
    }

    pub fn kani_lib_c(&self) -> Result<PathBuf> {
        self.base_path_with("library/kani/kani_lib.c")
    }

    pub fn kani_c_stubs(&self) -> Result<PathBuf> {
        self.base_path_with("library/kani/stubs/C")
    }

    pub fn cbmc_json_parser_py(&self) -> Result<PathBuf> {
        self.base_path_with("scripts/cbmc_json_parser.py")
    }

    pub fn kani_rlib(&self) -> Result<Option<PathBuf>> {
        match self {
            Self::DevRepo(_repo) => {
                // Awkwardly, there is not an easy way to determine the location of these outputs
                // So we let kani-compiler default to hard-coding them for development builds.
                Ok(None)
            }
            Self::Release(release) => {
                // First-time setup should place these here. Note `lib` not `library` for built artifacts.
                let path = release.join("lib");
                Ok(Some(expect_path(path)?))
            }
        }
    }

    /// A common case is that our repo and release bundle have the same `subpath`
    fn base_path_with(&self, subpath: &str) -> Result<PathBuf> {
        let path = match self {
            Self::DevRepo(r) => r,
            Self::Release(r) => r,
        };
        expect_path(path.join(subpath))
    }
}

/// A quick helper to say "hey, we expected this thing to be here but it's not!"
fn expect_path(path: PathBuf) -> Result<PathBuf> {
    if path.exists() {
        Ok(path)
    } else {
        bail!(
            "Unable to find {}. Looked for {}",
            path.file_name().unwrap().to_string_lossy(),
            path.display()
        );
    }
}
