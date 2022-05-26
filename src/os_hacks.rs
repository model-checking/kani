// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! In order to avoid introducing a large amount of OS-specific workarounds into the main
//! "flow" of code in setup.rs, this module contains all functions that implement os-specific
//! workarounds.

use std::process::Command;
use std::path::Path;
use std::ffi::OsString;

use anyhow::Result;

use crate::cmd::AutoRun;

/// See [`crate::setup::setup_python_deps`]
pub fn setup_python_deps_on_ubuntu_18_04(pyroot: &Path, pkg_versions: &[&str]) -> Result<()> {
    println!("Applying a workaround for 18.04...");
    // https://github.com/pypa/pip/issues/3826
    // Ubuntu 18.04 has a patched-to-be-broken version of pip that just straight-up makes `--target` not work.
    // Worse still, there is no apparent way to replicate the correct behavior cleanly.

    // This is a really awful hack to simulate getting the same result. I can find no other solution.
    // Example failed approach: `--system --target pyroot` fails to create a `pyroot/bin` with binaries.

    // Step 1: use `--system --prefix pyroot`. This disables the broken behavior, and creates `bin` but...
    Command::new("python3")
        .args(&["-m", "pip", "install", "--system", "--prefix"])
        .arg(&pyroot)
        .args(pkg_versions)
        .run()?;

    // Step 2: move `pyroot/lib/python3.6/site-packages/*` up to `pyroot`
    // This seems to successfully replicate the behavior of `--target`
    let mut mv_cmd = OsString::new();
    mv_cmd.push("mv ");
    mv_cmd.push(pyroot.as_os_str());
    mv_cmd.push("/lib/python*/site-packages/* ");
    mv_cmd.push(pyroot.as_os_str());
    Command::new("bash").arg("-c").arg(mv_cmd).run()?;

    Ok(())
}
