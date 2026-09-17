// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::InvocationType;
use crate::util;
use std::process::Command;

const KANI_RUST_VERIFIER: &str = "Kani Rust Verifier";
/// We assume this is the same as the `kani-verifier` version, but we should
/// make sure it's enforced through CI:
/// <https://github.com/model-checking/kani/issues/2626>
pub(crate) const KANI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The git revision Kani was built from, e.g. `kani-0.66.0-10-g21a9b31-dirty`.
/// Captured at build time by `build.rs`; empty when not built from a git
/// checkout (e.g. a released source tarball).
const KANI_GIT_REVISION: &str = env!("KANI_GIT_REVISION");
/// A summary of the rustc Kani was built with (and therefore uses), e.g.
/// `using rustc 1.93.0-nightly (29a69716f 2025-11-10) (commit 29a69716 2025-11-10) with LLVM 21.1.5`.
/// Captured at build time by `build.rs`; empty if it could not be determined.
const KANI_RUSTC_VERSION: &str = env!("KANI_RUSTC_VERSION");

/// Print Kani version. When `verbose` is true, this also appends the git build
/// revision (issue #2617) and the underlying rustc version (issue #2872).
pub(crate) fn print_kani_version(invocation_type: InvocationType, verbose: bool) {
    let kani_version = kani_version_release(invocation_type, verbose);
    println!("{kani_version}");

    if verbose && !KANI_RUSTC_VERSION.is_empty() {
        println!("{KANI_RUSTC_VERSION}");
    }
    // Callers gate this function on `--quiet`, so the pin check keeps the
    // tested zero-output contract of `--quiet`.
    print_cbmc_version_info();
}

const CBMC_VERSION_VAR: &str = "CBMC_VERSION";

/// Embedded at compile time because release bundles do not ship
/// `kani-dependencies`, so a runtime read would fail there.
const KANI_DEPENDENCIES: &str = include_str!("../../kani-dependencies");

/// The CBMC version found on `PATH`, or `None` if `cbmc` is absent or says
/// nothing. Single source of truth for the `cbmc --version` probe (also used by
/// `KaniSession::get_cbmc_info`).
pub(crate) fn cbmc_version_on_path() -> Option<String> {
    let output = Command::new("cbmc").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.split_whitespace().next().map(str::to_string)
}

fn pinned_cbmc_version() -> Option<String> {
    parse_dependency_var(KANI_DEPENDENCIES, CBMC_VERSION_VAR)
}

/// Extract `KEY=VALUE` (optionally quoted) from a shell-style assignment file.
fn parse_dependency_var(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    contents
        .lines()
        .find_map(|line| line.trim().strip_prefix(prefix.as_str()))
        .map(|value| value.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|value| !value.is_empty())
}

/// Print the `PATH` CBMC version. Warn, but do not fail, when it does not
/// match the pin: an unpinned CBMC must not block users.
fn print_cbmc_version_info() {
    let Some(found) = cbmc_version_on_path() else {
        return;
    };
    println!("CBMC {found}");

    if let Some(pinned) = pinned_cbmc_version()
        && let Some(warning) = cbmc_version_mismatch_warning(&found, &pinned)
    {
        util::warning(&warning);
    }
}

/// The mismatch warning, or `None` on a match. Split out for unit tests.
fn cbmc_version_mismatch_warning(found: &str, pinned: &str) -> Option<String> {
    if found == pinned {
        None
    } else {
        Some(format!(
            "found CBMC {found} on PATH, but Kani pins CBMC {pinned} (see `kani-dependencies`). \
             Verification results may not reflect the pinned toolchain."
        ))
    }
}

/// Print Kani release version as `Kani Rust Verifier <version>[ (<git-revision>)] (<invocation>)`
/// where:
///  - `<version>` is the `kani-verifier` version
///  - `<git-revision>` is the git build revision, included only when `verbose`
///    and available
///  - `<invocation>` is `cargo plugin` if Kani was invoked with `cargo kani` or
///    `standalone` if it was invoked with `kani`.
fn kani_version_release(invocation_type: InvocationType, verbose: bool) -> String {
    let invocation_str = match invocation_type {
        InvocationType::CargoKani(_) => "cargo plugin",
        InvocationType::Standalone => "standalone",
    };
    let git_info = if verbose && !KANI_GIT_REVISION.is_empty() {
        format!(" ({KANI_GIT_REVISION})")
    } else {
        String::new()
    };
    format!("{KANI_RUST_VERIFIER} {KANI_VERSION}{git_info} ({invocation_str})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_dependency_var() {
        let contents = "CBMC_MAJOR=\"6\"\nCBMC_VERSION=\"6.8.0\"\n\nKISSAT_VERSION=\"4.0.1\"\n";
        assert_eq!(parse_dependency_var(contents, "CBMC_VERSION"), Some("6.8.0".to_string()));
        assert_eq!(parse_dependency_var(contents, "KISSAT_VERSION"), Some("4.0.1".to_string()));
    }

    #[test]
    fn parses_unquoted_dependency_var() {
        assert_eq!(
            parse_dependency_var("CBMC_VERSION=6.8.0\n", "CBMC_VERSION"),
            Some("6.8.0".to_string())
        );
    }

    #[test]
    fn parses_single_quoted_dependency_var() {
        assert_eq!(
            parse_dependency_var("CBMC_VERSION='6.8.0'\n", "CBMC_VERSION"),
            Some("6.8.0".to_string())
        );
    }

    #[test]
    fn parses_real_kani_dependencies_file() {
        assert!(pinned_cbmc_version().is_some(), "kani-dependencies must define CBMC_VERSION");
    }

    #[test]
    fn missing_dependency_var_is_none() {
        assert_eq!(parse_dependency_var("KISSAT_VERSION=\"4.0.1\"\n", "CBMC_VERSION"), None);
    }

    #[test]
    fn empty_dependency_var_is_none() {
        assert_eq!(parse_dependency_var("CBMC_VERSION=\"\"\n", "CBMC_VERSION"), None);
    }

    #[test]
    fn mismatched_versions_produce_a_warning_naming_both() {
        let warning = cbmc_version_mismatch_warning("6.7.1", "6.8.0").unwrap();
        assert!(warning.contains("6.7.1"));
        assert!(warning.contains("6.8.0"));
    }

    #[test]
    fn matching_versions_produce_no_warning() {
        assert_eq!(cbmc_version_mismatch_warning("6.8.0", "6.8.0"), None);
    }
}
