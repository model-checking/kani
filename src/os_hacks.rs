// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! In order to avoid introducing a large amount of OS-specific workarounds into the main
//! "flow" of code in setup.rs, this module contains all functions that implement os-specific
//! workarounds.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use os_info::Info;

use crate::cmd::AutoRun;

pub fn check_minimum_python_version(output: &str) -> Result<bool> {
    // Split the string by whitespace and get the second element
    let version_number = output.split_whitespace().nth(1).unwrap_or("Version number not found");
    let parts: Vec<&str> = version_number.split('.').take(2).collect();
    let system_python_version = parts.join(".");

    // The minimum version is set to be 3.7 for now
    // TODO: Maybe read from some config file instead of a local variable?
    let base_version = "3.7";

    match compare_versions(&system_python_version, base_version) {
        Ok(ordering) => match ordering {
            std::cmp::Ordering::Less => Ok(false),
            std::cmp::Ordering::Equal => Ok(true),
            std::cmp::Ordering::Greater => Ok(true),
        },
        Err(_e) => Ok(false)
    }
}

// Given two semver strings, compare them and return an std::Ordering result
fn compare_versions(version1: &str, version2: &str) -> Result<std::cmp::Ordering, String> {
    let v1_parts: Vec<i32> = version1.split('.').map(|s| s.parse::<i32>().unwrap()).collect();
    let v2_parts: Vec<i32> = version2.split('.').map(|s| s.parse::<i32>().unwrap()).collect();

    let max_len = std::cmp::max(v1_parts.len(), v2_parts.len());

    // Compare semver strings by comparing each individual substring
    // to corresponding counterpart. i.e major version vs major version and so on
    for i in 0..max_len {
        let part_v1 = *v1_parts.get(i).unwrap_or(&0);
        let part_v2 = *v2_parts.get(i).unwrap_or(&0);

        if part_v1 != part_v2 {
            return Ok(part_v1.cmp(&part_v2));
        }
    }

    Ok(std::cmp::Ordering::Equal)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_greater() {
        assert_eq!(compare_versions("3.7.1", "3.6.3"), Ok(std::cmp::Ordering::Greater));
    }

    #[test]
    fn version_less() {
        assert_eq!(compare_versions("3.7.1", "3.7.3"), Ok(std::cmp::Ordering::Less));
    }

    #[test]
    fn version_equal() {
        assert_eq!(compare_versions("3.6.3", "3.6.3"), Ok(std::cmp::Ordering::Equal));
    }

    #[test]
    fn version_different_len() {
        assert_eq!(compare_versions("4.0", "4.0.0"), Ok(std::cmp::Ordering::Equal));
    }
}
