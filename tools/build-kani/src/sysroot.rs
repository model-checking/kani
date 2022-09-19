// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module has all the logic to build Kani's sysroot and libraries.
//! Rustc expects the sysroot to have a specific folder layout:
//! {SYSROOT}/rustlib/<target-triplet>/lib/<libraries>
//!
//! Note: We don't cross-compile. Target is the same as the host.

// SCRIPT_DIR="// ( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
// ROOT_DIR=// (dirname "$SCRIPT_DIR")
//
// # We don't cross-compile. Target is the same as the host.
// TARGET=// (rustc -vV | awk '/^host/ { print $2 }')
// TARGET_DIR="// {ROOT_DIR}/target/library-build"
// OUT_DIR="// {1:-"${ROOT_DIR}/target/lib"}"
// # Rust toolchain expects a specific format.
// STD_OUT_DIR="// {OUT_DIR}/rustlib/${TARGET}/lib/"
// mkdir -p "// {TARGET_DIR}"
// mkdir -p "// {OUT_DIR}"
// mkdir -p "// {STD_OUT_DIR}"
//
// # Build Kani libraries with custom std.
// cd "// {ROOT_DIR}"
// # note: build.hostflags isn't working.
// RUSTFLAGS="-Z always-encode-mir --cfg=kani" \
//     cargo build -v -Z unstable-options \
//     --out-dir="// {OUT_DIR}" \
//     -Z target-applies-to-host \
//     -Z host-config \
//     -Z build-std=panic_abort,std,test \
//     --target // {TARGET} \
//     -p kani \
//     -p std \
//     -p kani_macros \
//     --target-dir "// {TARGET_DIR}" \
//     --profile dev \
//     --config 'profile.dev.panic="abort"' \
//     --config 'host.rustflags=["--cfg=kani"]'
//
// # Copy std and dependencies to expected path.
// echo "Copy deps to // {OUT_DIR}"
// cp -r "// {TARGET_DIR}"/${TARGET}/debug/deps/*rlib "${OUT_DIR}"
//
// # Link to src
// STD_SRC="// (rustc --print sysroot)/lib/rustlib/src"
// ln -f -s "// STD_SRC" "${OUT_DIR}/rustlib/src"

use crate::{cp, cp_files, AutoRun};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build_sysroot() {
    // Run cargo build with -Z build-std
    let target = env!("TARGET");
    let target_dir = env!("KANI_DEV_LIBS");
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
    let sysroot_folder = Path::new(env!("KANI_DEV_SYSROOT"));
    sysroot_folder.exists().then(|| fs::remove_dir_all(sysroot_folder));
    fs::create_dir(sysroot_folder).expect(&format!("Failed to create {:?}", sysroot_folder));

    //  Copy Kani libraries to inside sysroot folder.
    let target_folder = Path::new(target_dir);
    let mut kani_macros = PathBuf::new();
    kani_macros.push(target_folder);
    kani_macros.push("debug");
    kani_macros.push("libkani_macros.so");
    assert!(kani_macros.exists(), "Cannot find {:?}", kani_macros);
    cp(&kani_macros, sysroot_folder).unwrap();

    let mut kani_rlib_folder = PathBuf::new();
    kani_rlib_folder.push(target_folder);
    kani_rlib_folder.push(target);
    kani_rlib_folder.push("debug");
    assert!(kani_macros.exists(), "Cannot find {:?}", kani_rlib_folder);
    cp_files(&kani_rlib_folder, sysroot_folder, &is_rlib).unwrap();

    // Copy `std` libraries and dependencies to sysroot folder following expected path format.
    // TODO: Create a macro for all these push.
    let mut src_path = PathBuf::new();
    src_path.push(target_folder);
    src_path.push(target);
    src_path.push("debug");
    src_path.push("deps");

    let mut dst_path = PathBuf::new();
    dst_path.push(sysroot_folder);
    dst_path.push("rustlib");
    dst_path.push(target);
    dst_path.push("lib");
    fs::create_dir_all(&dst_path).unwrap();
    cp_files(&src_path, &dst_path, &is_rlib).unwrap();
}

fn is_rlib(path: &Path) -> bool {
    path.is_file() && String::from(path.file_name().unwrap().to_string_lossy()).ends_with(".rlib")
}
