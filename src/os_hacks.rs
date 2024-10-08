// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! In order to avoid introducing a large amount of OS-specific workarounds into the main
//! "flow" of code in setup.rs, this module contains all functions that implement os-specific
//! workarounds.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use os_info::Info;

use crate::cmd::AutoRun;

/// This is the final step of setup, where we look for OSes that require additional setup steps
/// beyond the usual ones that we have done already.
pub fn setup_os_hacks(kani_dir: &Path, os: &Info) -> Result<()> {
    match os.os_type() {
        os_info::Type::NixOS => setup_nixos_patchelf(kani_dir),
        os_info::Type::Linux => {
            // NixOs containers are detected as Unknown Linux, so use a fallback hack:
            if std::env::var_os("NIX_CC").is_some() && Path::new("/etc/nix").exists() {
                return setup_nixos_patchelf(kani_dir);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// On NixOS, the dynamic linker does not live at the standard path, and so our downloaded
/// pre-built binaries need patching.
/// In addition, the C++ standard library (needed by the CBMC binaries we ship) also does not
/// have a standard path, and so we need to inject an rpath into those binaries to get them
/// to successfully link at runtime.
fn setup_nixos_patchelf(kani_dir: &Path) -> Result<()> {
    // Encode our assumption that we're working on x86 here, because when we add ARM
    // support, we need to look for a different path.
    // Prevents clippy error.
    let target = "x86_64-unknown-linux-gnu";
    assert!(env!("TARGET") == target);
    if Path::new("/lib64/ld-linux-x86-64.so.2").exists() {
        // if the expected path exists, I guess things are fine?
        return Ok(());
    }

    println!("[NixOS detected] Applying 'patchelf' to downloaded binaries");

    // Find the correct dynamic linker:
    // `interp=$(cat $NIX_CC/nix-support/dynamic-linker)`
    let nix_cc = std::env::var_os("NIX_CC")
        .context("On NixOS but 'NIX_CC` environment variable not set, couldn't apply patchelf.")?;
    let path = Path::new(&nix_cc).join("nix-support/dynamic-linker");
    let interp_raw = std::fs::read_to_string(path)
        .context("Couldn't read $NIX_CC/nix-support/dynamic-linker")?;
    let interp = interp_raw.trim();

    // Find the correct path to link C++ stdlib:
    // `rpath=$(nix-instantiate --eval -E "(import <nixpkgs> {}).stdenv.cc.cc.lib.outPath")/lib`
    let rpath_output = Command::new("nix-instantiate")
        .args(["--eval", "-E", "(import <nixpkgs> {}).stdenv.cc.cc.lib.outPath"])
        .output()?;
    if !rpath_output.status.success() {
        bail!("Failed to find C++ standard library with `nix-instantiate`");
    }
    let rpath_raw = std::str::from_utf8(&rpath_output.stdout)?;
    // The output is in quotes, remove them:
    let rpath_prefix = rpath_raw.trim().trim_matches('"');
    let rpath = format!("{rpath_prefix}/lib");

    let patch_interp = |file: &Path| -> Result<()> {
        Command::new("patchelf").args(["--set-interpreter", interp]).arg(file).run()
    };
    let patch_rpath = |file: &Path| -> Result<()> {
        Command::new("patchelf").args(["--set-rpath", &rpath]).arg(file).run()
    };

    let bin = kani_dir.join("bin");

    for filename in &["kani-compiler", "kani-driver"] {
        patch_interp(&bin.join(filename))?;
    }
    for filename in &["cbmc", "goto-analyzer", "goto-cc", "goto-instrument", "kissat", "symtab2gb"]
    {
        let file = bin.join(filename);
        patch_interp(&file)?;
        patch_rpath(&file)?;
    }

    Ok(())
}
