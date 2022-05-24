// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This crate includes two "proxy binaries": `kani` and `cargo-kani`.
//! These are conveniences to make it easy to:
//!
//! ```bash
//! cargo install --locked kani-verifer
//! ```
//!
//! Upon first run, or upon running `cargo-kani setup`, these proxy
//! binaries will download the appropriate Kani release bundle and invoke
//! the "real" `kani` and `cargo-kani` binaries.

#![warn(clippy::all, clippy::cargo)]

mod cmd;
mod setup;

use std::env;
use std::ffi::OsString;
use std::os::unix::prelude::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Effectively the entry point (i.e. `main` function) for both our proxy binaries.
/// `bin` should be either `kani` or `cargo-kani`
pub fn proxy(bin: &str) -> Result<()> {
    // In an effort to keep our dependencies minimal, we do the bare minimum argument parsing
    let args: Vec<_> = env::args_os().collect();
    if args.len() >= 2 && args[1] == "setup" {
        if args.len() >= 4 && args[2] == "--use-local-bundle" {
            setup::setup(Some(args[3].clone()))
        } else {
            setup::setup(None)
        }
    } else {
        fail_if_in_dev_environment()?;
        if !setup::appears_setup() {
            setup::setup(None)?;
        }
        exec(bin)
    }
}

/// In dev environments, this proxy shouldn't be used.
/// But accidentally using it (with the test suite) can fire off
/// hundreds of HTTP requests trying to download a non-existent release bundle.
/// So if we positively detect a dev environment, raise an error early.
fn fail_if_in_dev_environment() -> Result<()> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(path) = exe.parent() {
            if path.ends_with("target/debug") || path.ends_with("target/release") {
                bail!(
                    "Running a release-only executable, {}, from a development environment. This is usually caused by PATH including 'target/release' erroneously.",
                    exe.file_name().unwrap().to_string_lossy()
                )
            }
        }
    }

    Ok(())
}

/// Executes `kani-driver` in `bin` mode (kani or cargo-kani)
/// augmenting environment variables to accomodate our release environment
fn exec(bin: &str) -> Result<()> {
    let kani_dir = setup::kani_dir();
    let program = kani_dir.join("bin").join("kani-driver");
    let pyroot = kani_dir.join("pyroot");
    let bin_kani = kani_dir.join("bin");
    let bin_pyroot = pyroot.join("bin");
    let bin_toolchain = kani_dir.join("toolchain").join("bin");

    // Allow python scripts to find dependencies under our pyroot
    let pythonpath = prepend_search_path(&[pyroot], env::var_os("PYTHONPATH"))?;
    // Add: kani, cbmc, viewer (pyroot), and our rust toolchain directly to our PATH
    let path = prepend_search_path(&[bin_kani, bin_pyroot, bin_toolchain], env::var_os("PATH"))?;

    let mut cmd = Command::new(program);
    cmd.args(env::args_os().skip(1)).env("PYTHONPATH", pythonpath).env("PATH", path).arg0(bin);

    let result = cmd.status().context("Failed to invoke kani-driver")?;

    std::process::exit(result.code().expect("No exit code?"));
}

/// Prepend paths to an environment variable search string like PATH
fn prepend_search_path(paths: &[PathBuf], original: Option<OsString>) -> Result<OsString> {
    match original {
        None => Ok(env::join_paths(paths)?),
        Some(original) => {
            let orig = env::split_paths(&original);
            let new_iter = paths.iter().cloned().chain(orig);
            Ok(env::join_paths(new_iter)?)
        }
    }
}
