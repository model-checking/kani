// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file is a glorified shell script for constructing a Kani release bundle.
//! We use Rust here just to aid in making the "script" more robust.
//!
//! Run with `cargo run -p make-kani-release -- <version>` and this will produce
//! (e.g.) `kani-1.0-x86_64-unknown-linux-gnu.tar.gz`.

use std::{ffi::OsString, path::Path, process::Command};

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let version_string = parse_args()?;
    let kani_string = format!("kani-{}", version_string);
    let bundle_name = format!("{}-{}.tar.gz", kani_string, env!("TARGET"));
    let dir = Path::new(&kani_string);

    // Check everything is ready before we start copying files
    prebundle(dir)?;

    std::fs::create_dir(dir)?;

    bundle_kani(dir)?;
    bundle_cbmc(dir)?;
    // cbmc-viewer isn't bundled, it's pip install'd on first-time setup

    create_release_bundle(dir, &bundle_name)?;

    std::fs::remove_dir_all(dir)?;

    println!("\nSuccessfully built release bundle: {}", bundle_name);
    Ok(())
}

/// Parse command line arguments, and return the only thing we expect: a version string
fn parse_args() -> Result<String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        bail!("Usage: cargo run -p make-kani-release -- <version>");
    }
    Ok(args[1].clone())
}

/// Ensures everything is good to go before we begin to build the release bundle.
/// Notably, builds Kani in release mode.
fn prebundle(dir: &Path) -> Result<()> {
    if !Path::new("src/kani-compiler").exists() {
        bail!("Run from project root directory. Couldn't find 'src/kani-compiler'.");
    }

    if dir.exists() {
        bail!(
            "Directory {} already exists. Previous failed run? Delete it first.",
            dir.to_string_lossy()
        );
    }

    if !which::which("cbmc").is_ok() {
        bail!("Couldn't find the 'cbmc' binary to include in the release bundle.");
    }

    // Before we begin, ensure Kani is built successfully in release mode.
    Command::new("cargo").args(&["build", "--release"]).run()?;

    // TODO: temporarily, until cargo-kani is renamed kani-driver, we need to build this thing not in our workspace.
    // This isn't actually used by this script, but by the Dockerfile that tests this script
    Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(Path::new("src/kani-verifier"))
        .run()?;

    Ok(())
}

/// Copy Kani files into `dir`
fn bundle_kani(dir: &Path) -> Result<()> {
    let bin = dir.join("bin");
    std::fs::create_dir(&bin)?;

    // 1. Kani binaries
    let release = Path::new("./target/release");
    cp(&release.join("cargo-kani"), &bin)?;
    cp(&release.join("kani-compiler"), &bin)?;

    // 2. Kani scripts
    let scripts = dir.join("scripts");
    std::fs::create_dir(&scripts)?;

    cp(Path::new("./scripts/cbmc_json_parser.py"), &scripts)?;

    // 3. Kani libraries
    let library = dir.join("library");
    std::fs::create_dir(&library)?;

    cp_dir(Path::new("./library/kani"), &library)?;
    cp_dir(Path::new("./library/kani_macros"), &library)?;
    cp_dir(Path::new("./library/std"), &library)?;

    // 4. Record the exact toolchain we use
    std::fs::write(dir.join("rust-toolchain-version"), env!("RUSTUP_TOOLCHAIN"))?;

    // 5. Include a licensing note
    cp(Path::new("tools/make-kani-release/license-notes.txt"), dir)?;

    Ok(())
}
/// Copy CBMC files into `dir`
fn bundle_cbmc(dir: &Path) -> Result<()> {
    // In an effort to avoid creating new places where we must specify the exact version
    // of CBMC in use, we use the version in PATH here. This isn't ideal because it means
    // our release script is not standalone in determining how the release bundle is created.
    // We depend on other scripts to set up our environment correctly first.
    // This means it's possible to erroneously use this script, which is not ideal. Fool-proof is best.
    // But the best fix would involve changing our CI process to do something like
    // "make-kani-release" and then using *that* to run the test suite.
    // That way, we could just specify here what versions to use, and not need it in other places.

    // I felt that would be too invasive of a change to make at this time, so we'll start
    // with this approach and refactor it later.

    let bin = dir.join("bin");

    // We use these directly
    cp(&which::which("cbmc")?, &bin)?;
    cp(&which::which("goto-instrument")?, &bin)?;
    cp(&which::which("goto-cc")?, &bin)?;
    cp(&which::which("symtab2gb")?, &bin)?;
    // cbmc-viewer invokes this
    cp(&which::which("goto-analyzer")?, &bin)?;

    Ok(())
}

/// Create the release tarball from `./dir` named `bundle`.
/// This should include all files as `dir/<path>` in the tarball.
/// e.g. `kani-1.0/bin/kani-compiler` not just `bin/kani-compiler`.
fn create_release_bundle(dir: &Path, bundle: &str) -> Result<()> {
    Command::new("tar").args(&["zcf", bundle]).arg(dir).run()
}

/// Helper trait to fallibly run commands
trait AutoRun {
    fn run(&mut self) -> Result<()>;
}
impl AutoRun for Command {
    fn run(&mut self) -> Result<()> {
        let status = self.status()?;
        if !status.success() {
            bail!("Failed command: {}", render_command(self).to_string_lossy());
        }
        Ok(())
    }
}

/// Copy a single file to a directory
fn cp(src: &Path, dst: &Path) -> Result<()> {
    if !dst.is_dir() {
        bail!("{} isn't a directory", dst.to_string_lossy());
    }
    let dst = dst.join(src.file_name().unwrap());
    std::fs::copy(src, dst)?;
    Ok(())
}
/// Invoke `cp -r`
fn cp_dir(src: &Path, dst: &Path) -> Result<()> {
    let mut cmd = OsString::from("cp -r ");
    cmd.push(src.as_os_str());
    cmd.push(" ");
    cmd.push(dst.as_os_str());

    Command::new("bash").arg("-c").arg(cmd).run()
}

/// Render a Command as a string, to log it
pub fn render_command(cmd: &Command) -> OsString {
    let mut str = OsString::new();

    for (k, v) in cmd.get_envs() {
        if let Some(v) = v {
            str.push(k);
            str.push("=\"");
            str.push(v);
            str.push("\" ");
        }
    }

    str.push(cmd.get_program());

    for a in cmd.get_args() {
        str.push(" ");
        if a.to_string_lossy().contains(' ') {
            str.push("\"");
            str.push(a);
            str.push("\"");
        } else {
            str.push(a);
        }
    }

    str
}
