// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! In order to avoid introducing a large amount of OS-specific workarounds into the main
//! "flow" of code in setup.rs, this module contains all functions that implement os-specific
//! workarounds.

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use os_info::Info;

use crate::cmd::AutoRun;

pub fn should_apply_ubuntu_18_04_python_hack(os: &os_info::Info) -> Result<bool> {
    if os.os_type() != os_info::Type::Ubuntu {
        return Ok(false);
    }
    // Check both versions: https://github.com/stanislav-tkach/os_info/issues/318
    if *os.version() != os_info::Version::Semantic(18, 4, 0)
        && *os.version() != os_info::Version::Custom("18.04".into())
    {
        return Ok(false);
    }
    // It's not enough to check that we're on Ubuntu 18.04 because the user may have
    // manually updated to a newer version of Python instead of using what the OS ships.
    // So check if it looks like the OS-shipped version as best we can.
    let cmd = Command::new("python3").args(["-m", "pip", "--version"]).output()?;
    let output = std::str::from_utf8(&cmd.stdout)?;
    // The problem version looks like:
    //    'pip 9.0.1 from /usr/lib/python3/dist-packages (python 3.6)'
    // So we'll test for version 9.
    Ok(pip_major_version(output)? == 9)
}

/// Unit testable parsing function for extracting pip version numbers, from strings that look like:
///    'pip 9.0.1 from /usr/lib/python3/dist-packages (python 3.6)'
fn pip_major_version(output: &str) -> Result<u32> {
    // We don't want dependencies so parse with stdlib string functions as best we can.
    let mut words = output.split_whitespace();
    let _pip = words.next().context("No pip output")?;
    let version = words.next().context("No pip version")?;

    let mut versions = version.split('.');
    let major = versions.next().context("No pip major version")?;

    Ok(major.parse()?)
}

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
    // TODO clippy detects this as statically true and complains
    // assert!(env!("TARGET") == "x86_64-unknown-linux-gnu");
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
        .args(&["--eval", "-E", "(import <nixpkgs> {}).stdenv.cc.cc.lib.outPath"])
        .output()?;
    if !rpath_output.status.success() {
        bail!("Failed to find C++ standard library with `nix-instantiate`");
    }
    let rpath_raw = std::str::from_utf8(&rpath_output.stdout)?;
    // The output is in quotes, remove them:
    let rpath_prefix = rpath_raw.trim().trim_matches('"');
    let rpath = format!("{rpath_prefix}/lib");

    let patch_interp = |file: &Path| -> Result<()> {
        Command::new("patchelf").args(&["--set-interpreter", interp]).arg(file).run()
    };
    let patch_rpath = |file: &Path| -> Result<()> {
        Command::new("patchelf").args(&["--set-rpath", &rpath]).arg(file).run()
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
    fn check_pip_major_version() -> Result<()> {
        // These read a lot better formatted on one line, so shorten them:
        use pip_major_version as p;
        // 18.04 example: (with extra newline to test whitespace handling)
        assert_eq!(p("pip 9.0.1 from /usr/lib/python3/dist-packages (python 3.6)\n")?, 9);
        // a mac
        assert_eq!(p("pip 21.1.1 from /usr/local/python3.9/site-packages/pip (python 3.9)")?, 21);
        // 20.04
        assert_eq!(p("pip 20.0.2 from /usr/lib/python3/dist-packages/pip (python 3.8)")?, 20);
        // How mangled can we get and still "work"?
        assert_eq!(p("pip 1")?, 1);
        assert_eq!(p("p 1")?, 1);
        assert_eq!(p("\n\n p 1 p")?, 1);
        Ok(())
    }
}
