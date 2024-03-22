// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module has all the logic to build Kani's sysroot folder.
//! In this folder, you can find the following folders:
//! - `bin/`: Where all Kani binaries will be located.
//! - `lib/`: Kani libraries as well as rust standard libraries.
//!
//! Rustc expects the sysroot to have a specific folder layout:
//! `{SYSROOT}/rustlib/<target-triplet>/lib/<libraries>`
//!
//! Note: We don't cross-compile. Target is the same as the host.

use crate::{cp, AutoRun};
use anyhow::{bail, format_err, Result};
use cargo_metadata::{Artifact, Message};
use std::ffi::OsStr;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

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
    "so"
}

#[cfg(target_os = "macos")]
fn lib_extension() -> &'static str {
    "dylib"
}

/// Returns the path to Kani sysroot. I.e.: folder where we store pre-compiled binaries and
/// libraries.
pub fn kani_sysroot() -> PathBuf {
    PathBuf::from(env!("KANI_SYSROOT"))
}

/// Returns the path to where Kani and std pre-compiled libraries are stored.
pub fn kani_sysroot_lib() -> PathBuf {
    path_buf!(kani_sysroot(), "lib")
}

/// Returns the path to where Kani libraries for concrete playback is kept.
pub fn kani_playback_lib() -> PathBuf {
    path_buf!(kani_sysroot(), "playback/lib")
}

/// Returns the path to where Kani's pre-compiled binaries are stored.
fn kani_sysroot_bin() -> PathBuf {
    path_buf!(kani_sysroot(), "bin")
}

/// Build the `lib/` folder and `lib-playback/` for the new sysroot.
/// - The `lib/` folder contains the sysroot for verification.
/// - The `lib-playback/` folder contains the sysroot used for playback.
pub fn build_lib(bin_folder: &Path) -> Result<()> {
    let compiler_path = bin_folder.join("kani-compiler");
    build_verification_lib(&compiler_path)?;
    build_playback_lib(&compiler_path)
}

/// Build the `lib/` folder for the new sysroot used during verification.
/// This will include Kani's libraries as well as the standard libraries compiled with --emit-mir.
fn build_verification_lib(compiler_path: &Path) -> Result<()> {
    let extra_args =
        ["-Z", "build-std=panic_abort,std,test", "--config", "profile.dev.panic=\"abort\""];
    let compiler_args = ["--kani-compiler", "-Cllvm-args=--ignore-global-asm --build-std"];
    build_kani_lib(compiler_path, &kani_sysroot_lib(), &extra_args, &compiler_args)
}

/// Build the `lib-playback/` folder that will be used during counter example playback.
/// This will include Kani's libraries compiled with `concrete-playback` feature enabled.
fn build_playback_lib(compiler_path: &Path) -> Result<()> {
    let extra_args =
        ["--features=std/concrete_playback,kani/concrete_playback", "-Z", "build-std=std,test"];
    build_kani_lib(compiler_path, &kani_playback_lib(), &extra_args, &[])
}

fn build_kani_lib(
    compiler_path: &Path,
    output_path: &Path,
    extra_cargo_args: &[&str],
    extra_rustc_args: &[&str],
) -> Result<()> {
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
        "--profile",
        "dev",
        // Disable debug assertions for now as a mitigation for
        // https://github.com/model-checking/kani/issues/1740
        "--config",
        "profile.dev.debug-assertions=false",
        "--config",
        "host.rustflags=[\"--cfg=kani\", \"--cfg=kani_sysroot\"]",
        "--target",
        target,
        "--message-format",
        "json-diagnostic-rendered-ansi",
    ];
    let mut rustc_args = vec![
        "--cfg=kani",
        "--cfg=kani_sysroot",
        "-Z",
        "always-encode-mir",
        "-Z",
        "mir-enable-passes=-RemoveStorageMarkers",
    ];
    rustc_args.extend_from_slice(extra_rustc_args);
    let mut cmd = Command::new("cargo")
        .env("CARGO_ENCODED_RUSTFLAGS", rustc_args.join("\x1f"))
        .env("RUSTC", compiler_path)
        .args(args)
        .args(extra_cargo_args)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run `cargo build`.");

    // Collect the build artifacts.
    let artifacts = build_artifacts(&mut cmd);
    let exit_status = cmd.wait().expect("Couldn't get cargo's exit status");
    // `exit_ok` is an experimental API where we could do `.exit_ok().expect("...")` instead of the
    // below use of `panic`.
    if !exit_status.success() {
        bail!("Build failed: `cargo build-dev` didn't complete successfully");
    }

    // Create sysroot folder hierarchy.
    copy_artifacts(&artifacts, output_path, target)
}

/// Copy all the artifacts to their correct place to generate a valid sysroot.
fn copy_artifacts(artifacts: &[Artifact], sysroot_lib: &Path, target: &str) -> Result<()> {
    // Create sysroot folder hierarchy.
    sysroot_lib.exists().then(|| fs::remove_dir_all(sysroot_lib));
    let std_path = path_buf!(&sysroot_lib, "rustlib", target, "lib");
    fs::create_dir_all(&std_path).expect(&format!("Failed to create {std_path:?}"));

    //  Copy Kani libraries into sysroot top folder.
    copy_libs(&artifacts, &sysroot_lib, &is_kani_lib);
    //  Copy standard libraries into rustlib/<target>/lib/ folder.
    copy_libs(&artifacts, &std_path, &is_std_lib);
    Ok(())
}

/// Check if an artifact is a rust library that can be used by rustc on further crates compilations.
/// This inspects the kind of targets that this artifact originates from.
fn is_rust_lib(artifact: &Artifact) -> bool {
    artifact.target.kind.iter().any(|kind| match kind.as_str() {
        "lib" | "rlib" | "proc-macro" => true,
        "bin" | "dylib" | "cdylib" | "staticlib" | "custom-build" => false,
        _ => unreachable!("Unknown crate type {kind}"),
    })
}

/// Return whether this a kani library.
/// For a given artifact, check if this is a library or proc_macro, and whether this is a local
/// crate, i.e., that it is part of the Kani repository.
fn is_kani_lib(artifact: &Artifact) -> bool {
    is_rust_lib(artifact) && artifact.target.src_path.starts_with(env!("KANI_REPO_ROOT"))
}

/// Is this a std library or one of its dependencies.
/// For a given artifact, check if this is a library or proc_macro, and whether its source does
/// not belong to a Kani library.
fn is_std_lib(artifact: &Artifact) -> bool {
    is_rust_lib(artifact) && !is_kani_lib(artifact)
}

/// Copy the library files from the artifacts that match the given `predicate`.
/// This function will iterate over the list of artifacts generated by the compiler, it will
/// filter the artifacts according to the given predicate. For the artifacts that satisfy the
/// predicate, it will copy the following files to the `target` folder.
///  - `rlib`: Store metadata for future codegen and executable code for concrete executions.
///  - shared library which are used for proc_macros.
fn copy_libs<P>(artifacts: &[Artifact], target: &Path, mut predicate: P)
where
    P: FnMut(&Artifact) -> bool,
{
    assert!(target.is_dir(), "Expected a folder, but found {}", target.display());
    for artifact in artifacts.iter().filter(|&x| predicate(x)).cloned() {
        artifact
            .filenames
            .iter()
            .filter(|path| {
                path.extension() == Some("rlib") || path.extension() == Some(lib_extension())
            })
            .for_each(|lib| cp(lib.clone().as_std_path(), target).unwrap());
    }
}

/// Collect all the artifacts generated by Cargo build.
/// This will also include libraries that didn't need to be rebuild.
fn build_artifacts(cargo_cmd: &mut Child) -> Vec<Artifact> {
    let reader = BufReader::new(cargo_cmd.stdout.take().unwrap());
    Message::parse_stream(reader)
        .filter_map(|message| {
            match message.unwrap() {
                Message::CompilerMessage(msg) => {
                    // Print message as cargo would.
                    println!("{msg}");
                    None
                }
                Message::CompilerArtifact(artifact) => Some(artifact),
                Message::BuildScriptExecuted(_) | Message::BuildFinished(_) => {
                    // do nothing
                    None
                }
                // Non-exhaustive enum.
                _ => None,
            }
        })
        .collect()
}

/// Build Kani binaries with the extra arguments provided and return the path to the binaries folder.
/// Extra arguments to be given to `cargo build` while building Kani's binaries.
/// Note that the following arguments are always provided:
/// ```bash
/// cargo build --bins -Z unstable-options --out-dir $KANI_SYSROOT/bin/
/// ```
pub fn build_bin<T: AsRef<OsStr>>(extra_args: &[T]) -> Result<PathBuf> {
    let out_dir = kani_sysroot_bin();
    let args = ["--bins", "-Z", "unstable-options", "--out-dir", out_dir.to_str().unwrap()];
    Command::new("cargo")
        .arg("build")
        .args(extra_args)
        .args(args)
        .run()
        .or(Err(format_err!("Failed to build binaries.")))?;
    Ok(out_dir)
}
