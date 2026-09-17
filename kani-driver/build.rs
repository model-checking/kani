// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env::var;
use std::process::Command;

fn main() {
    // We want to know what target triple we were built with, but this isn't normally provided to us.
    // Note the difference between:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    // So "repeat" the info from build script (here) to our crate's build environment.
    println!("cargo:rustc-env=TARGET={}", var("TARGET").unwrap());

    // Capture the git revision so `kani --version --verbose` can report the
    // development build it was built from (see issue #2617). Empty when not
    // building from a git checkout (e.g. a released source tarball).
    let git_revision = run("git", &["describe", "--tags", "--always", "--dirty=-dirty"])
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=KANI_GIT_REVISION={git_revision}");
    // Rebuild the version string when the git state changes.
    if let Some(git_dir) = run("git", &["rev-parse", "--git-dir"]).map(|s| s.trim().to_string()) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/index");
    }

    // Capture the version of the exact rustc Kani is built with (and therefore
    // uses) so `kani --version --verbose` can report it (see issue #2872).
    // Using the build-time compiler is more reliable than shelling out to
    // `rustc` at runtime, which would report whatever toolchain happens to be
    // active in the user's environment rather than the one Kani bundles.
    let rustc = var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let rustc_info = run(&rustc, &["--version", "--verbose"])
        .map(|out| format_rustc_version(&out))
        .unwrap_or_default();
    println!("cargo:rustc-env=KANI_RUSTC_VERSION={rustc_info}");
}

/// Run `cmd args...` and return its stdout on success, or `None` on any failure.
fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if output.status.success() { String::from_utf8(output.stdout).ok() } else { None }
}

/// Turn `rustc --version --verbose` output into a one-line summary such as
/// `using rustc 1.96.0-nightly (80381278a 2026-03-01) (commit 80381278 2026-03-01) with LLVM 22.1.0`.
fn format_rustc_version(verbose_output: &str) -> String {
    let mut rustc_version = None;
    let mut commit_hash = None;
    let mut commit_date = None;
    let mut llvm_version = None;
    for line in verbose_output.lines() {
        if line.starts_with("rustc ") {
            rustc_version = Some(line.trim());
        } else if let Some(hash) = line.strip_prefix("commit-hash: ") {
            commit_hash = Some(hash.trim());
        } else if let Some(date) = line.strip_prefix("commit-date: ") {
            commit_date = Some(date.trim());
        } else if let Some(llvm) = line.strip_prefix("LLVM version: ") {
            llvm_version = Some(llvm.trim());
        }
    }
    let mut result = String::new();
    if let Some(version) = rustc_version {
        result.push_str(&format!("using {version}"));
    }
    if let (Some(hash), Some(date)) = (commit_hash, commit_date) {
        result.push_str(&format!(" (commit {} {date})", &hash[..8.min(hash.len())]));
    }
    if let Some(llvm) = llvm_version {
        result.push_str(&format!(" with LLVM {llvm}"));
    }
    result
}
