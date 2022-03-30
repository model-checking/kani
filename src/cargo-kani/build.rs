// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env::var;

fn main() {
    // We want to know what target triple we were built with, but this isn't normally provided to us.
    // Note the difference between:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    // So "repeat" the info from build script (here) to our crate's build environment.
    println!("cargo:rustc-env=TARGET={}", var("TARGET").unwrap());

    // rustup also seems to set some environment variables, but this is not clearly documented.
    // https://github.com/rust-lang/rustup/blob/master/src/toolchain.rs (search for RUSTUP_HOME)
    // We make use of RUSTUP_TOOLCHAIN in this crate, to avoid needing to otherwise repeat this information
    // from our `rust-toolchain.toml` file here.
}
