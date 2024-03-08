// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env;
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

/// Configure the compiler to build kani-compiler binary. We currently support building
/// kani-compiler with nightly only. We also link to the rustup rustc_driver library for now.
pub fn main() {
    // Add rustup to the rpath in order to properly link with the correct rustc version.

    // This is for dev purposes only, if dev point/search toolchain in .rustup/toolchains/
    let rustup_home = env::var("RUSTUP_HOME").unwrap();
    let rustup_tc = env::var("RUSTUP_TOOLCHAIN").unwrap();
    let rustup_lib = path_str!([&rustup_home, "toolchains", &rustup_tc, "lib"]);
    println!("cargo:rustc-link-arg-bin=kani-compiler=-Wl,-rpath,{rustup_lib}");

    // While we hard-code the above for development purposes, for a release/install we look
    // in a relative location for a symlink to the local rust toolchain
    let origin = if cfg!(target_os = "macos") { "@loader_path" } else { "$ORIGIN" };
    println!("cargo:rustc-link-arg-bin=kani-compiler=-Wl,-rpath,{origin}/../toolchain/lib");
}
