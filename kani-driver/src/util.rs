// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module that provides functions which are convenient for different purposes.
//!
//! In particular, the `warning` and `error` functions must be used for
//! diagnostic output across the `kani-driver` components. Please follow the
//! recommendations in <https://model-checking.github.io/kani/conventions.html>
//! when reporting any kind of diagnostic for users. Note that it's recommended
//! to use the Rust compiler's error message utilities if you're working on the
//! `kani-compiler`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Replace an extension with another one, in a new PathBuf. (See tests for examples)
pub fn alter_extension(path: &Path, ext: &str) -> PathBuf {
    path.with_extension(ext)
}

/// Generate a valid crate name from the input file.
/// Note that this method will replace invalid characters from the crate name.
pub fn crate_name(path: &Path) -> String {
    let stem = path.file_stem().unwrap().to_str().expect("utf-8 filename");
    stem.replace(['-', '.'], "_")
}

/// Given a path of some sort (usually from argv0), this attempts to extract the basename / stem
/// of the executable. e.g. "/path/foo -> foo" "./foo.exe -> foo" "foo -> foo"
pub fn executable_basename(argv0: &Option<&OsString>) -> Option<OsString> {
    if let Some(path) = argv0 {
        let basename = Path::new(&path).file_stem();
        if let Some(stem) = basename {
            return Some(stem.to_os_string());
        }
    }
    None
}

/// Render a Command as a string, to log it (e.g. in dry runs)
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

/// Print a warning message. This will add a "warning:" tag before the message and style accordingly.
pub fn warning(msg: &str) {
    let warning = console::style("warning:").bold().yellow();
    let msg_fmt = console::style(msg).bold();
    println!("{warning} {msg_fmt}")
}

/// Print an error message. This will add an "error:" tag before the message and style accordingly.
pub fn error(msg: &str) {
    let error = console::style("error:").bold().red();
    let msg_fmt = console::style(msg).bold();
    println!("{error} {msg_fmt}")
}

/// Print an info message. This will print the stage in bold green and the rest in regular style.
pub fn info_operation(op: &str, msg: &str) {
    let op_fmt = console::style(op).bold().green();
    let msg_fmt = console::style(msg);
    println!("{op_fmt} {msg_fmt}")
}

pub(crate) mod args {
    use std::{
        ffi::{OsStr, OsString},
        process::Command,
    };

    #[derive(Clone, PartialEq)]
    /// Kani-specific arguments passed to rustc and then used by `kani-compiler`.
    /// This includes everything from the `Arguments` struct in `kani-compiler/src/args.rs`.
    pub struct KaniArg(String);

    #[derive(Clone, PartialEq)]
    /// Arguments passed to rustc.
    /// Basically anything that gets listed when running `rustc --help`.
    /// This includes options like `--extern` for specify extern crates, `-L` for adding to the lib
    /// search path or `--emit mir` for emitting MIR.
    pub struct RustcArg(OsString);

    #[derive(Clone, PartialEq)]
    /// Arguments passed to Cargo.
    /// Basically anything that gets listed when running `cargo --help`. This includes options like
    /// all cargo subcommands, `--config`, `--target`, or any `-Z` unstable Cargo options.
    pub struct CargoArg(OsString);

    macro_rules! from_impl {
        ($type:tt, $inner:ty) => {
            impl<T> From<T> for $type
            where
                T: Into<$inner>,
            {
                fn from(value: T) -> Self {
                    $type(value.into())
                }
            }

            impl $type {
                /// Get a reference to this argument's underlying type.
                pub fn as_inner(&self) -> &$inner {
                    &self.0
                }
            }
        };
    }

    from_impl!(KaniArg, String);
    from_impl!(RustcArg, OsString);
    from_impl!(CargoArg, OsString);

    /// This function can be used to convert Kani compiler specific arguments into a rustc one.
    /// We currently pass Kani specific arguments using the `--llvm-args` structure which is the
    /// hacky mechanism used by other rustc backend to receive arguments unknown to rustc.
    ///
    /// Note that Cargo caching mechanism takes the building context into consideration, which
    /// includes the value of the rust flags. By using `--llvm-args`, we ensure that Cargo takes into
    /// consideration all arguments that are used to configure Kani compiler. For example, enabling the
    /// reachability checks will force recompilation if they were disabled in previous build.
    /// For more details on this caching mechanism, see the
    /// [fingerprint documentation](https://github.com/rust-lang/cargo/blob/82c3bb79e3a19a5164e33819ef81bfc2c984bc56/src/cargo/core/compiler/fingerprint/mod.rs)
    pub fn to_rustc_arg(kani_args: &[KaniArg]) -> RustcArg {
        format!(
            r#"-Cllvm-args={}"#,
            kani_args.iter().map(KaniArg::as_inner).cloned().collect::<Vec<String>>().join(" ")
        )
        .into()
    }

    pub enum PassTo {
        /// Only pass arguments for use in the local crate.
        /// This will just pass them directly as arguments to the command.
        OnlyLocalCrate,
        /// Pass arguments for use when compiling all dependencies using the
        /// `CARGO_ENCODED_RUSTFLAGS` environment variable.
        AllCrates,
    }

    pub trait CommandWrapper {
        fn pass_cargo_args(&mut self, args: &[CargoArg]) -> &mut Self;
        fn pass_rustc_args(&mut self, args: &[RustcArg], to: PassTo) -> &mut Self;
        fn pass_kani_args(&mut self, args: &[KaniArg], to: PassTo) -> &mut Self;
    }

    impl CommandWrapper for Command {
        /// Pass general arguments to cargo.
        fn pass_cargo_args(&mut self, args: &[CargoArg]) -> &mut Self {
            self.args(args.iter().map(CargoArg::as_inner))
        }

        /// Pass rustc arguments to the compiler for use in certain dependencies.
        fn pass_rustc_args(&mut self, args: &[RustcArg], to: PassTo) -> &mut Self {
            match to {
                // Since we also want to recursively pass these args to all dependencies,
                // use an environment variable that gets checked for each dependency.
                // Use of CARGO_ENCODED_RUSTFLAGS instead of RUSTFLAGS is preferred. See
                // https://doc.rust-lang.org/cargo/reference/environment-variables.html
                PassTo::AllCrates => self.env(
                    "CARGO_ENCODED_RUSTFLAGS",
                    args.iter()
                        .map(RustcArg::as_inner)
                        .cloned()
                        .collect::<Vec<OsString>>()
                        .join(OsStr::new("\x1f")),
                ),
                // Since we just want to pass to the local crate, just add them as arguments to the command.
                PassTo::OnlyLocalCrate => self.args(args.iter().map(RustcArg::as_inner)),
            }
        }

        /// Pass Kani-specific arguments to the compiler for use in certain dependencies.
        /// This will convert them to rustc args using the `--llvm-args` structure before
        /// adding them to the command to ensure they're properly parsed by the Kani compiler.
        fn pass_kani_args(&mut self, args: &[KaniArg], to: PassTo) -> &mut Self {
            self.pass_rustc_args(&[to_rustc_arg(args)], to)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_alter_extension() {
        let p = PathBuf::from("./path/file.rs");
        assert_eq!(alter_extension(&p, "symtab.json"), PathBuf::from("./path/file.symtab.json"));
        assert_eq!(
            alter_extension(&p, "kani-metadata.json"),
            PathBuf::from("./path/file.kani-metadata.json")
        );

        let q = PathBuf::from("file.more.rs");
        assert_eq!(alter_extension(&q, "symtab.json"), PathBuf::from("file.more.symtab.json"));
    }

    #[test]
    fn check_exe_basename() {
        assert_eq!(
            executable_basename(&Some(&OsString::from("/path/slash/foo"))),
            Some("foo".into())
        );
        assert_eq!(executable_basename(&Some(&OsString::from("./foo.exe"))), Some("foo".into()));
        assert_eq!(executable_basename(&Some(&OsString::from("foo.exe"))), Some("foo".into()));
        assert_eq!(executable_basename(&Some(&OsString::from("foo"))), Some("foo".into()));
    }

    #[test]
    fn check_render_command() {
        let mut c1 = Command::new("a");
        c1.arg("b");
        assert_eq!(render_command(&c1), OsString::from("a b"));
        c1.arg("/c d/");
        assert_eq!(render_command(&c1), OsString::from("a b \"/c d/\""));
        c1.env("PARAM", "VALUE");
        assert_eq!(render_command(&c1), OsString::from("PARAM=\"VALUE\" a b \"/c d/\""));
    }
}
