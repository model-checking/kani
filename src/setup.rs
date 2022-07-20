// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains all first-time setup code done as part of `cargo kani setup`.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::cmd::AutoRun;
use crate::os_hacks;

/// Comes from our Cargo.toml manifest file. Must correspond to our release verion.
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Set by our `build.rs`, reflects the Rust target triple we're building for
const TARGET: &str = env!("TARGET");

/// Where Kani has been installed. Typically `~/.kani/kani-1.x/`
pub fn kani_dir() -> PathBuf {
    home::home_dir()
        .expect("Couldn't find home dir?")
        .join(".kani")
        .join(format!("kani-{}", VERSION))
}

/// Fast check to see if we look setup already
pub fn appears_setup() -> bool {
    kani_dir().exists()
}

/// Sets up Kani by unpacking/installing to `~/.kani/kani-VERSION`
pub fn setup(use_local_bundle: Option<OsString>) -> Result<()> {
    let kani_dir = kani_dir();
    let os = os_info::get();

    println!("[0/6] Running Kani first-time setup...");

    println!("[1/6] Ensuring the existence of: {}", kani_dir.display());
    std::fs::create_dir_all(&kani_dir)?;

    setup_kani_bundle(&kani_dir, use_local_bundle)?;

    let toolchain_version = setup_rust_toolchain(&kani_dir)?;

    setup_python_deps(&kani_dir, &os)?;

    setup_build_kani_prelude(&kani_dir, toolchain_version)?;

    os_hacks::setup_os_hacks(&kani_dir, &os)?;

    println!("[6/6] Successfully completed Kani first-time setup.");

    Ok(())
}

/// Download and unpack the Kani release bundle
fn setup_kani_bundle(kani_dir: &Path, use_local_bundle: Option<OsString>) -> Result<()> {
    // e.g. `~/.kani/`
    let base_dir = kani_dir.parent().expect("No base directory?");

    if let Some(pathstr) = use_local_bundle {
        println!("[2/6] Installing local Kani bundle: {}", pathstr.to_string_lossy());
        let path = Path::new(&pathstr).canonicalize()?;
        // When given a local bundle, it's often "-latest" but we expect "-1.0" or something.
        // tar supports "stripping" the first directory from the bundle, so do that and
        // extract it directly into the expected (kani_dir) directory (instead of base_dir).
        Command::new("tar")
            .arg("--strip-components=1")
            .arg("-zxf")
            .arg(&path)
            .current_dir(&kani_dir)
            .run()?;
    } else {
        let filename = download_filename();
        println!("[2/6] Downloading Kani release bundle: {}", &filename);
        fail_if_unsupported_target()?;
        let bundle = base_dir.join(filename);
        Command::new("curl")
            .args(&["-sSLf", "-o"])
            .arg(&bundle)
            .arg(download_url())
            .run()
            .context("Failed to download Kani release bundle")?;

        Command::new("tar").arg("zxf").arg(&bundle).current_dir(base_dir).run()?;

        std::fs::remove_file(bundle)?;
    }
    Ok(())
}

/// Install the Rust toolchain version we require
fn setup_rust_toolchain(kani_dir: &Path) -> Result<String> {
    // Currently this means we require the bundle to have been unpacked first!
    let toolchain_version = std::fs::read_to_string(kani_dir.join("rust-toolchain-version"))
        .context("Reading release bundle rust-toolchain-version")?;
    println!("[3/6] Installing rust toolchain version: {}", &toolchain_version);
    Command::new("rustup").args(&["toolchain", "install", &toolchain_version]).run()?;

    let toolchain = home::rustup_home()?.join("toolchains").join(&toolchain_version);

    symlink_rust_toolchain(&toolchain, kani_dir)?;
    Ok(toolchain_version)
}

/// Install into the pyroot the python dependencies we need
fn setup_python_deps(kani_dir: &Path, os: &os_info::Info) -> Result<()> {
    println!("[4/6] Installing Kani python dependencies...");
    let pyroot = kani_dir.join("pyroot");

    // TODO: this is a repetition of versions from kani/kani-dependencies
    let pkg_versions = &["cbmc-viewer==3.5", "colorama==0.4.3"];

    if os.os_type() == os_info::Type::Ubuntu
        // Check both versions: https://github.com/stanislav-tkach/os_info/issues/318
        && (*os.version() == os_info::Version::Semantic(18, 4, 0)
            || *os.version() == os_info::Version::Custom("18.04".into()))
    {
        os_hacks::setup_python_deps_on_ubuntu_18_04(&pyroot, pkg_versions)?;
        return Ok(());
    }

    Command::new("python3")
        .args(&["-m", "pip", "install", "--target"])
        .arg(&pyroot)
        .args(pkg_versions)
        .run()?;

    Ok(())
}

/// Build the Kani prelude libaries locally
fn setup_build_kani_prelude(kani_dir: &Path, toolchain_version: String) -> Result<()> {
    println!("[5/6] Building Kani library prelude...");
    // We need a workspace to build them in, otherwise repeated builds generate different hashes and `kani` can't find `kani_macros`
    let contents = "[workspace]\nmembers = [\"kani\",\"kani_macros\",\"std\"]";
    std::fs::write(kani_dir.join("library").join("Cargo.toml"), contents)?;

    // A little helper for invoking Cargo repeatedly here
    let cargo = |crate_name: &str| -> Result<()> {
        let manifest = format!("library/{}/Cargo.toml", crate_name);
        Command::new("cargo")
            .args(&[
                &format!("+{}", toolchain_version),
                "build",
                "-Z",
                "unstable-options",
                "--manifest-path",
                &manifest,
                "--out-dir",
                "lib",
                "--target-dir",
                "target",
            ])
            .current_dir(&kani_dir)
            // https://doc.rust-lang.org/cargo/reference/environment-variables.html
            .env("CARGO_ENCODED_RUSTFLAGS", "--cfg=kani")
            .run()
            .with_context(|| format!("Failed to build Kani prelude library {}", crate_name))
    };

    // We seem to need 3 invocations because of the behavior of the `--out-dir` flag.
    // It only seems to produce the requested artifact, not its dependencies.
    cargo("kani")?;
    cargo("kani_macros")?;
    cargo("std")?;

    std::fs::remove_dir_all(kani_dir.join("target"))?;
    Ok(())
}

// This ends the setup steps above.
//
// Just putting a bit of space between that and the helper functions below.

/// The filename of the release bundle
fn download_filename() -> String {
    format!("kani-{}-{}.tar.gz", VERSION, TARGET)
}

/// The download URL for this version of Kani
fn download_url() -> String {
    let tag: &str = &format!("kani-{}", VERSION);
    let file: &str = &download_filename();
    format!("https://github.com/model-checking/kani/releases/download/{}/{}", tag, file)
}

/// Give users a better error message than "404" if we're on an unsupported platform.
/// This is called just before we try to download the release bundle.
fn fail_if_unsupported_target() -> Result<()> {
    // This is basically going to be reduced to a compile-time constant
    match TARGET {
        "x86_64-unknown-linux-gnu" | "x86_64-apple-darwin" => Ok(()),
        _ => bail!("Kani does not support this platform (Rust target {})", TARGET),
    }
}

/// Creates a `kani_dir/toolchain` symlink pointing to `toolchain`.
fn symlink_rust_toolchain(toolchain: &Path, kani_dir: &Path) -> Result<()> {
    let path = kani_dir.join("toolchain");
    // We want setup to be idempotent, so if the symlink already exists, delete instead of failing
    if path.exists() && path.is_symlink() {
        std::fs::remove_file(&path)?;
    }
    std::os::unix::fs::symlink(toolchain, path)?;
    Ok(())
}
