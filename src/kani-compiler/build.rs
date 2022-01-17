// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env;
use std::path::PathBuf;
use std::process::Command;

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

/// Build the target library, and setup cargo to rerun them if the source has changed.
fn setup_lib(out_dir: &str, lib_out: &str, lib: &str) {
    let rmc_lib = vec!["..", "..", "library", lib];
    println!("cargo:rerun-if-changed={}", path_str!(rmc_lib));

    let mut rmc_lib_toml = rmc_lib;
    rmc_lib_toml.push("Cargo.toml");
    let args = [
        "build",
        "--manifest-path",
        &path_str!(rmc_lib_toml),
        "-Z",
        "unstable-options",
        "--out-dir",
        lib_out,
        "--target-dir",
        &out_dir,
    ];
    Command::new("cargo").env("CARGO_ENCODED_RUSTFLAGS", "--cfg=rmc").args(args).status().unwrap();
}

/// Configure the compiler to build rmc-compiler binary. We currently support building
/// rmc-compiler with nightly only. We also link to the rustup rustc_driver library for now.
pub fn main() {
    // Add rustup to the rpath in order to properly link with the correct rustc version.
    let rustup_home = env::var("RUSTUP_HOME").unwrap();
    let rustup_tc = env::var("RUSTUP_TOOLCHAIN").unwrap();
    let rustup_lib = path_str!([&rustup_home, "toolchains", &rustup_tc, "lib"]);
    println!("cargo:rustc-link-arg-bin=rmc-compiler=-Wl,-rpath,{}", rustup_lib);

    // Compile rmc library and export RMC_LIB_PATH variable with its relative location.
    let out_dir = env::var("OUT_DIR").unwrap();
    let lib_out = path_str!([&out_dir, "lib"]);
    setup_lib(&out_dir, &lib_out, "rmc");
    setup_lib(&out_dir, &lib_out, "rmc_macros");
    println!("cargo:rustc-env=RMC_LIB_PATH={}", lib_out);
}
