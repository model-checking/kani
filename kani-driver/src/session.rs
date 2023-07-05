// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::args::common::Verbosity;
use crate::args::VerificationArgs;
use crate::util::render_command;
use anyhow::{bail, Context, Result};
use std::io::IsTerminal;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Mutex;
use std::time::Instant;
use strum_macros::Display;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// Environment variable used to control this session log tracing.
/// This is the same variable used to control `kani-compiler` logs. Note that you can still control
/// the driver logs separately, by using the logger directives to  select the kani-driver crate.
/// `export KANI_LOG=kani_driver=debug`.
const LOG_ENV_VAR: &str = "KANI_LOG";

/// Contains information about the execution environment and arguments that affect operations
pub struct KaniSession {
    /// The common command-line arguments
    pub args: VerificationArgs,

    /// Include all publicly-visible symbols in the generated goto binary, not just those reachable from
    /// a proof harness. Useful when attempting to verify things that were not annotated with kani
    /// proof attributes.
    pub codegen_tests: bool,

    /// The location we found the 'kani_rustc' command
    pub kani_compiler: PathBuf,
    /// The location we found 'kani_lib.c'
    pub kani_lib_c: PathBuf,
    /// The location we found the Kani C stub .c files
    pub kani_c_stubs: PathBuf,

    /// The temporary files we littered that need to be cleaned up at the end of execution
    pub temporaries: Mutex<Vec<PathBuf>>,
}

/// Represents where we detected Kani, with helper methods for using that information to find critical paths
pub enum InstallType {
    /// We're operating in a a checked out repo that's been built locally.
    /// The path here is to the root of the repo.
    DevRepo(PathBuf),
    /// We're operating from a release bundle (made with `build-kani release`).
    /// The path here to where this release bundle has been unpacked.
    Release(PathBuf),
}

impl KaniSession {
    pub fn new(args: VerificationArgs) -> Result<Self> {
        init_logger(&args);
        let install = InstallType::new()?;

        Ok(KaniSession {
            args,
            codegen_tests: false,
            kani_compiler: install.kani_compiler()?,
            kani_lib_c: install.kani_lib_c()?,
            kani_c_stubs: install.kani_c_stubs()?,
            temporaries: Mutex::new(vec![]),
        })
    }

    /// Record a temporary file so we can cleanup after ourselves at the end.
    /// Note that there will be no failure if the file does not exist.
    pub fn record_temporary_file<T: AsRef<Path>>(&self, temp: &T) {
        self.record_temporary_files(&[temp])
    }

    /// Record temporary files so we can cleanup after ourselves at the end.
    /// Note that there will be no failure if the file does not exist.
    pub fn record_temporary_files<T: AsRef<Path>>(&self, temps: &[T]) {
        // unwrap safety: will panic this thread if another thread panicked *while holding the lock.*
        // This is vanishingly unlikely, and even then probably the right thing to do
        let mut t = self.temporaries.lock().unwrap();
        t.extend(temps.iter().map(|p| p.as_ref().to_owned()));
    }

    /// Determine which symbols Kani should codegen (i.e. by slicing away symbols
    /// that are considered unreachable.)
    pub fn reachability_mode(&self) -> ReachabilityMode {
        if self.codegen_tests {
            ReachabilityMode::Tests
        } else if self.args.function.is_some() {
            ReachabilityMode::AllPubFns
        } else {
            ReachabilityMode::ProofHarnesses
        }
    }
}

#[derive(Debug, Copy, Clone, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ReachabilityMode {
    #[strum(to_string = "harnesses")]
    ProofHarnesses,
    #[strum(to_string = "pub_fns")]
    AllPubFns,
    Tests,
}

impl Drop for KaniSession {
    fn drop(&mut self) {
        if !self.args.keep_temps {
            let temporaries = self.temporaries.lock().unwrap();

            for file in temporaries.iter() {
                // If it fails, we don't care, skip it
                let _result = std::fs::remove_file(file);
            }
        }
    }
}

impl KaniSession {
    /// Call [run_terminal] with the verbosity configured by the user.
    pub fn run_terminal(&self, cmd: Command) -> Result<()> {
        run_terminal(&self.args.common_args, cmd)
    }

    /// Call [run_suppress] with the verbosity configured by the user.
    pub fn run_suppress(&self, cmd: Command) -> Result<()> {
        run_suppress(&self.args.common_args, cmd)
    }

    /// Call [run_redirect] with the verbosity configured by the user.
    pub fn run_redirect(&self, cmd: Command, stdout: &Path) -> Result<ExitStatus> {
        run_redirect(&self.args.common_args, cmd, stdout)
    }

    /// Call [run_piped] with the verbosity configured by the user.
    pub fn run_piped(&self, cmd: Command) -> Result<Option<Child>> {
        run_piped(&self.args.common_args, cmd)
    }

    /// Call [with_timer] with the verbosity configured by the user.
    pub fn with_timer<T, F>(&self, func: F, description: &str) -> T
    where
        F: FnOnce() -> T,
    {
        with_timer(&self.args.common_args, func, description)
    }
}

// The below suite of helper functions for executing Commands are meant to be a common handler
// for various cmdline flags like 'verbose' and 'quiet'. These functions are temporary: in the
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
pub fn run_terminal(verbosity: &impl Verbosity, mut cmd: Command) -> Result<()> {
    if verbosity.quiet() {
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
    }
    if verbosity.verbose() {
        println!("[Kani] Running: `{}`", render_command(&cmd).to_string_lossy());
    }
    let program = cmd.get_program().to_string_lossy().to_string();
    let result = with_timer(
        verbosity,
        || {
            cmd.status()
                .context(format!("Failed to invoke {}", cmd.get_program().to_string_lossy()))
        },
        &program,
    )?;
    if !result.success() {
        bail!("{} exited with status {}", cmd.get_program().to_string_lossy(), result);
    }
    Ok(())
}

/// Run a job, but only output (unless --quiet) if it fails, and fail if there's a problem.
pub fn run_suppress(verbosity: &impl Verbosity, mut cmd: Command) -> Result<()> {
    if verbosity.is_set() {
        return run_terminal(verbosity, cmd);
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
pub fn run_redirect(
    verbosity: &impl Verbosity,
    mut cmd: Command,
    stdout: &Path,
) -> Result<ExitStatus> {
    if verbosity.verbose() {
        println!(
            "[Kani] Running: `{} > {}`",
            render_command(&cmd).to_string_lossy(),
            stdout.display()
        );
    }
    let output_file = std::fs::File::create(&stdout)?;
    cmd.stdout(output_file);

    let program = cmd.get_program().to_string_lossy().to_string();
    with_timer(
        verbosity,
        || {
            cmd.status()
                .context(format!("Failed to invoke {}", cmd.get_program().to_string_lossy()))
        },
        &program,
    )
}

/// Run a job and pipe its output to this process.
/// Returns an error if the process could not be spawned.
///
/// NOTE: Unlike other `run_` functions, this function does not attempt to indicate
/// the process exit code, you need to remember to check this yourself.
pub fn run_piped(verbosity: &impl Verbosity, mut cmd: Command) -> Result<Option<Child>> {
    if verbosity.verbose() {
        println!("[Kani] Running: `{}`", render_command(&cmd).to_string_lossy());
    }
    // Run the process as a child process
    let process = cmd
        .stdout(Stdio::piped())
        .spawn()
        .context(format!("Failed to invoke {}", cmd.get_program().to_string_lossy()))?;

    Ok(Some(process))
}

/// Execute the provided function and measure the clock time it took for its execution.
/// Print the time with the given description if we are on verbose or debug mode.
fn with_timer<T, F>(verbosity: &impl Verbosity, func: F, description: &str) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let ret = func();
    if verbosity.verbose() {
        let elapsed = start.elapsed();
        println!("Finished {description} in {}s", elapsed.as_secs_f32())
    }
    ret
}

/// Return the path for the folder where the current executable is located.
fn bin_folder() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("Cannot determine current executable location")?;
    let dir = exe.parent().context("Executable isn't in a directory")?.to_owned();
    Ok(dir)
}

/// Return the path for the folder where the pre-compiled rust libraries are located.
pub fn lib_folder() -> Result<PathBuf> {
    Ok(base_folder()?.join("lib"))
}

/// Return the path for the folder where the pre-compiled rust libraries are located.
pub fn lib_playback_folder() -> Result<PathBuf> {
    Ok(base_folder()?.join("playback/lib"))
}

/// Return the base folder for the entire kani installation.
pub fn base_folder() -> Result<PathBuf> {
    Ok(bin_folder()?
        .parent()
        .context("Failed to find Kani's base installation folder.")?
        .to_path_buf())
}

impl InstallType {
    pub fn new() -> Result<Self> {
        // Case 1: We've checked out the development repo and we're built under `target/kani`
        let mut path = bin_folder()?;
        if path.ends_with("target/kani/bin") {
            path.pop();
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

/// Initialize the logger using the KANI_LOG environment variable and `--debug` argument.
fn init_logger(args: &VerificationArgs) {
    let filter = EnvFilter::from_env(LOG_ENV_VAR);
    let filter = if args.common_args.debug {
        filter.add_directive(LevelFilter::DEBUG.into())
    } else {
        filter
    };

    // Use a hierarchical view for now.
    let use_colors = std::io::stdout().is_terminal();
    let subscriber = Registry::default().with(filter);
    let subscriber = subscriber.with(
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(use_colors)
            .with_target(true),
    );
    tracing::subscriber::set_global_default(subscriber).unwrap();
}
