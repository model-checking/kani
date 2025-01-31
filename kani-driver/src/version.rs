// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::InvocationType;

const KANI_RUST_VERIFIER: &str = "Kani Rust Verifier";
/// We assume this is the same as the `kani-verifier` version, but we should
/// make sure it's enforced through CI:
/// <https://github.com/model-checking/kani/issues/2626>
pub(crate) const KANI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Print Kani version. At present, this is only release version information.
pub(crate) fn print_kani_version(invocation_type: InvocationType) {
    let kani_version = kani_version_release(invocation_type);
    // TODO: Print development version information.
    // <https://github.com/model-checking/kani/issues/2617>
    println!("{kani_version}");
}

/// Print Kani release version as `Kani Rust Verifier <version> (<invocation>)`
/// where:
///  - `<version>` is the `kani-verifier` version
///  - `<invocation>` is `cargo plugin` if Kani was invoked with `cargo kani` or
///    `standalone` if it was invoked with `kani`.
fn kani_version_release(invocation_type: InvocationType) -> String {
    let invocation_str = match invocation_type {
        InvocationType::CargoKani(_) => "cargo plugin",
        InvocationType::Standalone => "standalone",
    };
    format!("{KANI_RUST_VERIFIER} {KANI_VERSION} ({invocation_str})")
}
