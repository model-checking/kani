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

pub mod tempfile {
    use std::{
        env,
        fs::{self, rename, File},
        io::{BufWriter, Error, Write},
        path::PathBuf,
    };

    use crate::util;
    use ::rand;
    use anyhow::Context;
    use rand::Rng;

    /// Handle a writable temporary file which will be deleted when the object is dropped.
    /// To save the contents of the file, users can invoke `rename` which will move the file to
    /// its new location and no further deletion will be performed.
    pub struct TempFile {
        pub file: File,
        pub temp_path: PathBuf,
        pub writer: Option<BufWriter<File>>,
        renamed: bool,
    }

    impl TempFile {
        /// Create a temp file
        pub fn try_new(suffix_name: &str) -> Result<Self, Error> {
            let mut temp_path = env::temp_dir();

            // Generate a unique name for the temporary directory
            let hash: u32 = rand::thread_rng().gen();
            let file_name: &str = &format!("kani_tmp_{hash}_{suffix_name}");

            temp_path.push(file_name);
            let temp_file = File::create(&temp_path)?;
            let writer = BufWriter::new(temp_file.try_clone()?);

            Ok(Self { file: temp_file, temp_path, writer: Some(writer), renamed: false })
        }

        /// Rename the temporary file to the new path, replacing the original file if the path points to a file that already exists.
        pub fn rename(mut self, source_path: &str) -> Result<(), String> {
            // flush here
            self.writer.as_mut().unwrap().flush().unwrap();
            self.writer = None;
            // Renames are usually automic, so we won't corrupt the user's source file during a crash.
            rename(&self.temp_path, source_path)
                .with_context(|| format!("Error renaming file {}", self.temp_path.display()))
                .unwrap();
            self.renamed = true;
            Ok(())
        }
    }

    /// Ensure that the bufwriter is flushed and temp variables are dropped
    /// everytime the tempfile is out of scope
    /// note: the fields for the struct are dropped automatically by destructor
    impl Drop for TempFile {
        fn drop(&mut self) {
            // if writer is not flushed, flush it
            if self.writer.as_ref().is_some() {
                // couldn't use ? as drop does not handle returns
                if let Err(e) = self.writer.as_mut().unwrap().flush() {
                    util::warning(
                        format!("failed to flush {}: {e}", self.temp_path.display()).as_str(),
                    );
                }
                self.writer = None;
            }

            if !self.renamed {
                if let Err(_e) = fs::remove_file(self.temp_path.clone()) {
                    util::warning(&format!("Error removing file {}", self.temp_path.display()));
                }
            }
        }
    }
}

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

/// Attempt to guess the rlib name for rust source file.
/// This is only used by 'kani', never 'cargo-kani', so we hopefully don't have too many corner
/// cases to deal with.
/// In rustc, you can find some code dealing this this naming in:
///      compiler/rustc_codegen_ssa/src/back/link.rs
pub fn guess_rlib_name(path: &Path) -> PathBuf {
    let basedir = path.parent().unwrap_or(Path::new("."));
    let rlib_name = format!("lib{}.rlib", crate_name(path));

    basedir.join(rlib_name)
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
    fn check_guess_rlib_name() {
        assert_eq!(guess_rlib_name(Path::new("mycrate.rs")), PathBuf::from("libmycrate.rlib"));
        assert_eq!(guess_rlib_name(Path::new("my-crate.rs")), PathBuf::from("libmy_crate.rlib"));
        assert_eq!(guess_rlib_name(Path::new("./foo.rs")), PathBuf::from("./libfoo.rlib"));
        assert_eq!(guess_rlib_name(Path::new("a/b/foo.rs")), PathBuf::from("a/b/libfoo.rlib"));
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
