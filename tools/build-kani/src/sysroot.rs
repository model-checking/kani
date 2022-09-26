// Copyright Kani Contributors
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
use cargo_metadata::Message;
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
    ".so"
}

#[cfg(target_os = "macos")]
fn lib_extension() -> &'static str {
    ".dylib"
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

/// Returns the path to where Kani pre-compiled library are stored.
///
/// The legacy libraries are compiled on the top of rustup sysroot. Using it results in missing
/// symbols. This is still needed though because when we use the rust monomorphizer as our
/// reachability algorithm, the resulting boundaries are different than the new sysroot.
pub fn kani_sysroot_legacy_lib() -> PathBuf {
    path_buf!(kani_sysroot(), "legacy-lib")
}

/// Returns the path to where Kani's pre-compiled binaries are stored.
pub fn kani_sysroot_bin() -> PathBuf {
    path_buf!(kani_sysroot(), "bin")
}

/// Build the `lib/` folder for the new sysroot.
/// This will include Kani's libraries as well as the standard libraries compiled with --emit-mir.
/// TODO: Don't copy Kani's libstd.
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
        "--message-format",
        "json-diagnostic-rendered-ansi",
    ];
    let mut cmd = Command::new("cargo")
        .env("CARGO_ENCODED_RUSTFLAGS", ["--cfg=kani", "-Z", "always-encode-mir"].join("\x1f"))
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run `cargo build`.");

    // Remove kani "std" library leftover.
    filter_kani_std(&mut cmd);
    let _ = cmd.wait().expect("Couldn't get cargo's exit status");

    // Create sysroot folder.
    let sysroot_lib = kani_sysroot_lib();
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

/// Kani's "std" library may cause a name conflict with the rust standard library. We remove it
/// from the `deps/` folder, since we already store it outside of the `deps/` folder.
/// For that, we retrieve its location from cargo build output.
fn filter_kani_std(cargo_cmd: &mut Child) {
    let reader = BufReader::new(cargo_cmd.stdout.take().unwrap());
    for message in Message::parse_stream(reader) {
        match message.unwrap() {
            Message::CompilerMessage(msg) => {
                // Print message as cargo would.
                println!("{:?}", msg)
            }
            Message::CompilerArtifact(artifact) => {
                // Remote the `rlib` and `rmeta` kept in the deps folder.
                if artifact.target.name == "std"
                    && artifact.target.src_path.starts_with(env!("KANI_REPO_ROOT"))
                {
                    let rmeta = artifact.filenames.iter().find(|p| p.extension() == Some("rmeta"));
                    let mut glob = PathBuf::from(rmeta.unwrap());
                    glob.set_extension("*");
                    Command::new("rm").arg("-f").arg(glob.as_os_str()).run().unwrap();
                }
            }
            Message::BuildScriptExecuted(_script) => {
                // do nothing
            }
            Message::BuildFinished(_finished) => {
                // do nothing
            }
            // Non-exhaustive enum.
            _ => (),
        }
    }
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
    let legacy_lib = kani_sysroot_legacy_lib();
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

/// Extra arguments to be given to "cargo build" while building Kani's binaries.
/// Note that the following arguments are always provided:
/// --bins -Z unstable-options --out-dir $KANI_SYSROOT/bin/
pub fn build_bin<T: AsRef<OsStr>>(extra_args: &[T]) {
    let out_dir = kani_sysroot_bin();
    let args = ["--bins", "-Z", "unstable-options", "--out-dir", out_dir.to_str().unwrap()];
    Command::new("cargo")
        .arg("build")
        .args(args)
        .args(extra_args)
        .run()
        .expect("Failed to build binaries.");
}
