// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env::var;
use std::path::PathBuf;

macro_rules! path_str {
    ($input:expr) => {
        String::from(
            $input
                .iter()
                .collect::<PathBuf>()
                .to_str()
                .unwrap_or_else(|| panic!("Invalid path {}", stringify!($input))),
        )
    };
}

fn main() {
    // We want to know what target triple we were built with, but this isn't normally provided to us.
    // Note the difference between:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    // So "repeat" the info from build script (here) to our crate's build environment.
    let target_arch = var("TARGET").unwrap();
    let workspace_root = var("CARGO_WORKSPACE_DIR").unwrap();
    println!(
        "cargo:rustc-env=KANI_EXTERN_OUT_DIR={}",
        path_str!([workspace_root.as_ref(), "target", target_arch.as_ref(), "debug", "deps"]),
    );
    println!("cargo:rustc-env=TARGET={}", &target_arch);
}
