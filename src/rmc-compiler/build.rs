// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env;
use std::path::PathBuf;

/// Configure the compiler to build rmc-compiler binary. We currently support building
/// rmc-compiler with nightly only. We also link to the rustup rustc_driver library for now.
pub fn main() {
    // Add rustup to the rpath in order to properly link with the correct rustc version.
    let rustup_home = env::var("RUSTUP_HOME").unwrap();
    let rustup_tc = env::var("RUSTUP_TOOLCHAIN").unwrap();
    let lib_path = [&rustup_home, "toolchains", &rustup_tc, "lib"].iter().collect::<PathBuf>();
    println!("cargo:rustc-link-arg-bin=rmc-compiler=-Wl,-rpath,{}", lib_path.to_string_lossy());
}
