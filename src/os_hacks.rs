// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! In order to avoid introducing a large amount of OS-specific workarounds into the main
//! "flow" of code in setup.rs, this module contains all functions that implement os-specific
//! workarounds.

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use anyhow::{Result, Context, bail};
use os_info::Info;

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
    // "mv" is not idempotent however so we need to do "cp -r" then delete
    let mut cp_cmd = OsString::new();
    cp_cmd.push("cp -r ");
    cp_cmd.push(pyroot.as_os_str());
    cp_cmd.push("/lib/python*/site-packages/* ");
    cp_cmd.push(pyroot.as_os_str());
    Command::new("bash").arg("-c").arg(cp_cmd).run()?;

    // `lib` is the directory `--prefix` creates that `--target` does not.
    std::fs::remove_dir_all(pyroot.join("lib"))?;

    Ok(())
}

/// This is the final step of setup, where we look for OSes that require additional setup steps
/// beyond the usual ones that we have done already.
pub fn setup_os_hacks(kani_dir: &Path, os: &Info) -> Result<()> {
    match os.os_type() {
        os_info::Type::NixOS => setup_nixos_patchelf(kani_dir),
        _ => Ok(())
    }
}

/// On NixOS, the dynamic linker does not live at the standard path, and so our downloaded
/// pre-built binaries need patching.
fn setup_nixos_patchelf(kani_dir: &Path) -> Result<()> {
    // Encode our assumption that we're working on x86 here, because when we add ARM
    // support, we need to look for a different path.
    assert!(env!("TARGET") == "x86_64-unknown-linux-gnu");
    if Path::new("/lib64/ld-linux-x86-64.so.2").exists() {
        // if the expected path exists, I guess things are fine?
        return Ok(())
    }

    println!("[NixOS detected] Applying 'patchelf' to downloaded binaries");
    
    // patchelf --set-interpreter "$(cat $NIX_CC/nix-support/dynamic-linker)" ~/.kani/kani-0.1.0/bin/*
    if let Some(nix_cc) = std::env::var_os("NIX_CC") {
        let path = Path::new(&nix_cc).join("nix-support/dynamic-linker");
        let interp_raw = std::fs::read_to_string(path).context("Couldn't read $NIX_CC/nix-support/dynamic-linker")?;
        let interp = interp_raw.trim();

        let bin = kani_dir.join("bin");
        for entry in std::fs::read_dir(bin)? {
            let file = entry?;
            if file.file_type()?.is_file() {
                Command::new("patchelf").args(&["--set-interpreter", interp]).arg(file.file_name()).run()?;
            }
        }
    } else {
        bail!("On NixOS but 'NIX_CC` environment variable not set, couldn't apply patchelf.");
    }

    Ok(())
}

#[test]
fn check() {
    assert_eq!("", env!("TARGET"));
}
