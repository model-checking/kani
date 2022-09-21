// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module has all the logic to build Kani's sysroot folder.
//! In this folder, you can find the following folders:
//! - `bin/`: Where all Kani binaries will be located.
//! - `lib/`: Kani libraries as well as rust standard libraries.
//! - `legacy-lib/`: Kani libraries built based on the the toolchain standard libraries.
//!
//! Rustc expects the sysroot to have a specific folder layout:
//! {SYSROOT}/rustlib/<target-triplet>/lib/<libraries>
//!
//! Note: We don't cross-compile. Target is the same as the host.

use crate::{cp, cp_files, AutoRun};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

macro_rules! path_buf {
    // The arguments are expressions that can be pushed to the PathBuf.
    ($base_path:expr, $($extra_path:expr),+) => {{
        let mut path_buf = PathBuf::from($base_path);
        $(path_buf.push($extra_path);)+
        path_buf
    }};
}

#[cfg(target_os = "linux")]
fn lib_extension() -> &'static str {
    ".so"
}

#[cfg(target_os = "macos")]
fn lib_extension() -> &'static str {
    ".dylib"
}

pub fn kani_sysroot() -> PathBuf {
    PathBuf::from(env!("KANI_SYSROOT"))
}

pub fn build_lib() {
    // Run cargo build with -Z build-std
    let target = env!("TARGET");
    let target_dir = env!("KANI_BUILD_LIBS");
    let args = [
        "build",
        "-p",
        "std",
        "-p",
        "kani",
        "-p",
        "kani_macros",
        "-Z",
        "unstable-options",
        "--target-dir",
        target_dir,
        "-Z",
        "target-applies-to-host",
        "-Z",
        "host-config",
        "-Z",
        "build-std=panic_abort,std,test",
        "--profile",
        "dev",
        "--config",
        "profile.dev.panic=\"abort\"",
        "--config",
        "host.rustflags=[\"--cfg=kani\"]",
        "--target",
        target,
    ];
    Command::new("cargo")
        .env("CARGO_ENCODED_RUSTFLAGS", ["--cfg=kani", "-Z", "always-encode-mir"].join("\x1f"))
        .args(args)
        .run()
        .expect("Failed to build Kani libraries.");

    // Create sysroot folder.
    let sysroot_lib = path_buf!(kani_sysroot(), "lib");
    sysroot_lib.exists().then(|| fs::remove_dir_all(&sysroot_lib));
    fs::create_dir_all(&sysroot_lib).expect(&format!("Failed to create {:?}", sysroot_lib));

    //  Copy Kani libraries to inside sysroot folder.
    let target_folder = Path::new(target_dir);
    let macro_lib = format!("libkani_macros{}", lib_extension());
    let kani_macros = path_buf!(target_folder, "debug", macro_lib);
    cp(&kani_macros, &sysroot_lib).unwrap();

    let kani_rlib_folder = path_buf!(target_folder, target, "debug");
    cp_files(&kani_rlib_folder, &sysroot_lib, &is_rlib).unwrap();

    // Copy `std` libraries and dependencies to sysroot folder following expected path format.
    // TODO: Create a macro for all these push.
    let src_path = path_buf!(target_folder, target, "debug", "deps");

    let dst_path = path_buf!(sysroot_lib, "rustlib", target, "lib");
    fs::create_dir_all(&dst_path).unwrap();
    cp_files(&src_path, &dst_path, &is_rlib).unwrap();
}

/// Build Kani libraries using the regular rust toolchain standard libraries.
/// We should be able to remove this once the MIR linker is stable.
pub fn build_lib_legacy() {
    // Run cargo build with -Z build-std
    let target_dir = env!("KANI_LEGACY_LIBS");
    let args =
        ["build", "-p", "std", "-p", "kani", "-p", "kani_macros", "--target-dir", target_dir];
    Command::new("cargo")
        .env("CARGO_ENCODED_RUSTFLAGS", ["--cfg=kani"].join("\x1f"))
        .args(args)
        .run()
        .expect("Failed to build Kani libraries.");

    // Create sysroot folder.
    let legacy_lib = path_buf!(kani_sysroot(), "legacy-lib");
    legacy_lib.exists().then(|| fs::remove_dir_all(&legacy_lib));
    fs::create_dir_all(&legacy_lib).expect(&format!("Failed to create {:?}", legacy_lib));

    //  Copy Kani libraries to inside the lib folder.
    let target_folder = Path::new(target_dir);
    let macro_lib = format!("libkani_macros{}", lib_extension());
    let kani_macros = path_buf!(target_folder, "debug", macro_lib);
    cp(&kani_macros, &legacy_lib).unwrap();

    let kani_rlib_folder = path_buf!(target_folder, "debug");
    cp_files(&kani_rlib_folder, &legacy_lib, &is_rlib).unwrap();
}

fn is_rlib(path: &Path) -> bool {
    path.is_file() && String::from(path.file_name().unwrap().to_string_lossy()).ends_with(".rlib")
}

pub fn build_bin(args: &[&str]) {
    // Before we begin, ensure Kani is built successfully in release mode.
    Command::new("cargo").arg("build").args(args).run().expect("Failed to build binaries.");
}
