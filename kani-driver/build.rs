// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env::var;

fn main() {
    // We want to know what target triple we were built with, but this isn't normally provided to us.
    // Note the difference between:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    // So "repeat" the info from build script (here) to our crate's build environment.
    let target_arch = var("TARGET").unwrap();
    let workspace_root = var("CARGO_WORKSPACE_DIR").unwrap();
    println!(
        "cargo:rustc-env=KANI_EXTERN_OUT_DIR={}/target/{}/debug/deps",
        workspace_root, &target_arch
    );
    println!("cargo:rustc-env=TARGET={}", &target_arch);
}
