// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This crate includes two "proxy binaries": `kani` and `cargo-kani`.
//! These are conveniences to make it easy to:
//!
//! ```bash
//! cargo install --locked kani-verifer
//! ```
//!
//! Upon first run, or upon running `cargo-kani setup`, these proxy
//! binaries will download the appropriate Kani release bundle and invoke
//! the "real" `kani` and `cargo-kani` binaries.

#![warn(clippy::all, clippy::cargo)]

use std::env;
use std::ffi::OsString;
use std::os::unix::prelude::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Comes from our Cargo.toml manifest file. Must correspond to our release verion.
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Set by our `build.rs`, reflects the Rust target triple we're building for
const TARGET: &str = env!("TARGET");

/// Typically `~/.kani/kani-1.x/`
fn kani_dir() -> PathBuf {
    home::home_dir()
        .expect("Couldn't find home dir?")
        .join(".kani")
        .join(format!("kani-{}", VERSION))
}

/// The filename of the release bundle
fn download_filename() -> String {
    format!("kani-{}-{}.tar.gz", VERSION, TARGET)
}

/// Helper to find the download URL for this version of Kani
fn download_url() -> String {
    let tag: &str = &format!("kani-{}", VERSION);
    let file: &str = &download_filename();
    format!("https://github.com/model-checking/kani/releases/download/{}/{}", tag, file)
}

/// Effectively the entry point (i.e. `main` function) for both our proxy binaries.
pub fn proxy(bin: &str) -> Result<()> {
    // In an effort to keep our dependencies minimal, we do the bare minimum argument parsing
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() >= 2 && args[1] == "setup" {
        if args.len() >= 4 && args[2] == "--use-local-bundle" {
            setup(Some(args[3].clone()))
        } else {
            setup(None)
        }
    } else {
        fail_if_in_dev_environment()?;
        if !appears_setup() {
            setup(None)?;
        }
        exec(bin)
    }
}

/// Fast check to see if we look setup already
fn appears_setup() -> bool {
    kani_dir().exists()
}

/// In dev environments, this proxy shouldn't be used.
/// But accidentally using it (with the test suite) can fire off
/// hundreds of HTTP requests trying to download a non-existent release bundle.
/// So if we positively detect a dev environment, raise an error early.
fn fail_if_in_dev_environment() -> Result<()> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(path) = exe.parent() {
            if path.ends_with("target/debug") || path.ends_with("target/release") {
                bail!(
                    "Running a release-only executable, {}, from a development environment. This is usually caused by PATH including 'target/release' erroneously.",
                    exe.file_name().unwrap().to_string_lossy()
                )
            }
        }
    }

    Ok(())
}

/// Give users a better error message than "404" if we're on an unsupported platform.
fn fail_if_unsupported_target() -> Result<()> {
    // This is basically going to be reduced to a compile-time constant
    match TARGET {
        "x86_64-unknown-linux-gnu" | "x86_64-apple-darwin" => Ok(()),
        _ => bail!("Kani does not support this platform (Rust target {})", TARGET),
    }
}

/// Sets up Kani by unpacking/installing to `~/.kani/kani-VERSION`
fn setup(use_local_bundle: Option<OsString>) -> Result<()> {
    let kani_dir = kani_dir();
    // e.g. `~/.kani/`
    let base_dir = kani_dir.parent().expect("No base directory?");

    println!("[0/6] Running Kani first-time setup...");

    println!("[1/6] Ensuring the existence of: {}", base_dir.display());
    std::fs::create_dir_all(&base_dir)?;

    if let Some(pathstr) = use_local_bundle {
        let path = Path::new(&pathstr).canonicalize()?;
        println!("[2/6] Installing local Kani bundle: {}", path.display());
        Command::new("tar").arg("zxf").arg(&path).current_dir(base_dir).run()?;

        // when given a local bundle, it's often "-latest" but we expect "-1.0" or something. Hack it up.
        let file = path.file_name().expect("has filename").to_string_lossy();
        let components: Vec<_> = file.split('-').collect();
        let expected_dir = format!("{}-{}", components[0], components[1]);

        std::fs::rename(base_dir.join(expected_dir), &kani_dir)?;
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

    let toolchain_version = std::fs::read_to_string(kani_dir.join("rust-toolchain-version"))
        .context("Reading release bundle rust-toolchain-version")?;
    println!("[3/6] Installing rust toolchain version: {}", &toolchain_version);
    Command::new("rustup").args(&["toolchain", "install", &toolchain_version]).run()?;

    let toolchain = home::rustup_home()?.join("toolchains").join(&toolchain_version);

    Command::new("ln").arg("-s").arg(toolchain).arg(kani_dir.join("toolchain")).run()?;

    println!("[4/6] Installing Kani python dependencies...");
    let pyroot = kani_dir.join("pyroot");

    // TODO: this is a repetition of versions from elsewhere
    Command::new("python3")
        .args(&["-m", "pip", "install", "cbmc-viewer==2.11", "--target"])
        .arg(&pyroot)
        .run()?;
    Command::new("python3")
        .args(&["-m", "pip", "install", "colorama==0.4.3", "--target"])
        .arg(&pyroot)
        .run()?;

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

    println!("[6/6] Successfully completed Kani first-time setup.");

    Ok(())
}

/// Executes `kani-driver` in `bin` mode (kani or cargo-kani)
/// augmenting environment variables to accomodate our release environment
fn exec(bin: &str) -> Result<()> {
    let kani_dir = kani_dir();
    let program = kani_dir.join("bin").join("kani-driver");
    let pyroot = kani_dir.join("pyroot");
    let bin_kani = kani_dir.join("bin");
    let bin_pyroot = pyroot.join("bin");
    let bin_toolchain = kani_dir.join("toolchain").join("bin");

    // Allow python scripts to find dependencies under our pyroot
    let pythonpath = augment_search(&[pyroot], env::var_os("PYTHONPATH"))?;
    // Add: kani, cbmc, viewer (pyroot), and our rust toolchain directly to our PATH
    let path = augment_search(&[bin_kani, bin_pyroot, bin_toolchain], env::var_os("PATH"))?;

    let mut cmd = Command::new(program);
    cmd.args(std::env::args_os().skip(1)).env("PYTHONPATH", pythonpath).env("PATH", path).arg0(bin);

    let result = cmd.status().context("Failed to invoke kani-driver")?;

    std::process::exit(result.code().expect("No exit code?"));
}

/// Prepend paths to an environment variable
fn augment_search(paths: &[PathBuf], original: Option<OsString>) -> Result<OsString> {
    match original {
        None => Ok(env::join_paths(paths)?),
        Some(original) => {
            let orig = env::split_paths(&original);
            let new_iter = paths.iter().cloned().chain(orig);
            Ok(env::join_paths(new_iter)?)
        }
    }
}

/// Helper trait to fallibly run commands
trait AutoRun {
    fn run(&mut self) -> Result<()>;
}
impl AutoRun for Command {
    fn run(&mut self) -> Result<()> {
        // This can sometimes fail during the set-up of the forked process before exec,
        // for example by setting `current_dir` to a directory that does not exist.
        let status = self.status().with_context(|| {
            format!(
                "Internal failure before invoking command: {}",
                render_command(self).to_string_lossy()
            )
        })?;
        if !status.success() {
            bail!("Failed command: {}", render_command(self).to_string_lossy());
        }
        Ok(())
    }
}

/// Render a Command as a string, to log it
pub fn render_command(cmd: &Command) -> OsString {
    let mut str = OsString::new();

    for (k, v) in cmd.get_envs() {
        if let Some(v) = v {
            str.push(k);
            str.push("=\"");
            str.push(v);
            str.push("\" ");
        }
    }

    str.push(cmd.get_program());

    for a in cmd.get_args() {
        str.push(" ");
        if a.to_string_lossy().contains(' ') {
            str.push("\"");
            str.push(a);
            str.push("\"");
        } else {
            str.push(a);
        }
    }

    str
}
