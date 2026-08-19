// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::InvocationType;

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
