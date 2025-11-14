// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::InvocationType;
use anyhow::Result;
use build_kani::built_info;
use std::process::Command;

const KANI_RUST_VERIFIER: &str = "Kani Rust Verifier";
/// We assume this is the same as the `kani-verifier` version, but we should
/// make sure it's enforced through CI:
/// <https://github.com/model-checking/kani/issues/2626>
pub(crate) const KANI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Print Kani version. When verbose is true, also includes rustc version information.
pub(crate) fn print_kani_version(invocation_type: InvocationType, verbose: bool) {
    let kani_version = kani_version_release(invocation_type, verbose);
    println!("{kani_version}");

    if verbose {
        if let Ok(rustc_info) = get_rustc_version_info() {
            println!("{rustc_info}");
        } else {
            println!("rustc version information unavailable");
        }
    }
}

/// Print Kani release version as `Kani Rust Verifier <version> (<invocation>)`
/// where:
///  - `<version>` is the `kani-verifier` version
///  - `<invocation>` is `cargo plugin` if Kani was invoked with `cargo kani` or
///    `standalone` if it was invoked with `kani`.
fn kani_version_release(invocation_type: InvocationType, verbose: bool) -> String {
    let invocation_str = match invocation_type {
        InvocationType::CargoKani(_) => "cargo plugin",
        InvocationType::Standalone => "standalone",
    };
    let git_info_opt = if verbose && let Some(git_version) = built_info::GIT_VERSION {
        if built_info::GIT_DIRTY == Some(true) {
            format!(" ({git_version}-dirty)")
        } else {
            format!(" ({git_version})")
        }
    } else {
        "".to_string()
    };
    format!("{KANI_RUST_VERIFIER} {KANI_VERSION}{git_info_opt} ({invocation_str})")
}

/// Get rustc version and commit information
fn get_rustc_version_info() -> Result<String> {
    let output = Command::new("rustc").arg("--version").arg("--verbose").output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to get rustc version");
    }

    let version_output = String::from_utf8(output.stdout)?;
    let lines: Vec<&str> = version_output.lines().collect();

    // Parse the verbose output to extract relevant information
    let mut rustc_version = None;
    let mut commit_hash = None;
    let mut commit_date = None;
    let mut llvm_version = None;

    for line in lines {
        if line.starts_with("rustc ") {
            rustc_version = Some(line.trim());
        } else if line.starts_with("commit-hash: ") {
            commit_hash = Some(line.trim_start_matches("commit-hash: ").trim());
        } else if line.starts_with("commit-date: ") {
            commit_date = Some(line.trim_start_matches("commit-date: ").trim());
        } else if line.starts_with("LLVM version: ") {
            llvm_version = Some(line.trim_start_matches("LLVM version: ").trim());
        }
    }

    let mut result = String::new();
    if let Some(version) = rustc_version {
        result.push_str(&format!("using {}", version));
    }
    if let (Some(hash), Some(date)) = (commit_hash, commit_date) {
        result.push_str(&format!(" (commit {} {})", &hash[..8.min(hash.len())], date));
    }
    if let Some(llvm) = llvm_version {
        result.push_str(&format!(" with LLVM {}", llvm));
    }

    Ok(result)
}
