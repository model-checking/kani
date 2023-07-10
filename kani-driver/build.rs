// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env::var;

use cargo_metadata::MetadataCommand;

fn main() {
    // We want to know what target triple we were built with, but this isn't normally provided to us.
    // Note the difference between:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    // So "repeat" the info from build script (here) to our crate's build environment.
    println!("cargo:rustc-env=TARGET={}", var("TARGET").unwrap());

    // Force this script to re-run if `kani-verifier/Cargo.toml` changed.
    // Otherwise, we may not get the up-to-date version in `kani-driver`.
    println!("cargo:rerun-if-changed=../Cargo.toml");

    // Pass the `kani-verifier` version through the build environment.
    println!("cargo:rustc-env=KANI_VERIFIER_VERSION={}", kani_verifier_version());
}

/// Retrieve the `kani-verifier` version using `cargo metadata`
fn kani_verifier_version() -> String {
    let metadata = MetadataCommand::new().no_deps().exec().expect("failed to obtain metadata");
    // Find the `kani-verifier` package metadata and return the version
    let kani_verifier_metadata = metadata.packages.iter().find(|p| p.name == "kani-verifier");
    kani_verifier_metadata.unwrap().version.to_string()
}
