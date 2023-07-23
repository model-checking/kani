// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{determine_invocation_type, InvocationType};

const KANI_RUST_VERIFIER: &str = "Kani Rust Verifier";
const KANI_VERIFIER_VERSION: &str = env!("KANI_VERIFIER_VERSION");

/// Print Kani version. At present, this is only release version information.
pub(crate) fn print_kani_version() -> String {
    // TODO: Print development version information.
    // <https://github.com/model-checking/kani/issues/2617>
    kani_version_release()
}

/// Print Kani release version as `Kani Rust Verifier <version> (<invocation>)`
/// where:
///  - `<version>` is the `kani-verifier` version
///  - `<invocation>` is `cargo plugin` if Kani was invoked with `cargo kani` or
///    `standalone` if it was invoked with `kani`.
fn kani_version_release() -> String {
    let mut version_str = "Kani Rust Verifier ".to_string();
    version_str.push_str(KANI_VERIFIER_VERSION);

    let invocation_str = match determine_invocation_type(Vec::from_iter(std::env::args_os())) {
        InvocationType::CargoKani(_) => "cargo plugin",
        InvocationType::Standalone => "standalone",
    };
    format!("{KANI_RUST_VERIFIER} {KANI_VERIFIER_VERSION} ({invocation_str})")
}
