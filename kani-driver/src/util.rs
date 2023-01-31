// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

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

/// Joining an OsString with a delimeter is missing from Rust libraries, so
/// let's write out own, and with convenient types...
pub fn join_osstring(elems: &[OsString], joiner: &str) -> OsString {
    let mut str = OsString::new();
    for (i, arg) in elems.iter().enumerate() {
        if i != 0 {
            str.push(OsString::from(joiner));
        }
        str.push(arg);
    }
    str
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

/// Generate the filename for a specialized harness from the base linked object
pub fn specialized_harness_name(linked_obj: &Path, harness_filename: &str) -> PathBuf {
    alter_extension(linked_obj, &format!("for-{harness_filename}.out"))
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
    fn check_join_osstring() {
        assert_eq!(
            join_osstring(&["a".into(), "b".into(), "cd".into()], " "),
            OsString::from("a b cd")
        );
        assert_eq!(join_osstring(&[], " "), OsString::from(""));
        assert_eq!(join_osstring(&["a".into()], " "), OsString::from("a"));
        assert_eq!(
            join_osstring(&["a".into(), "b".into(), "cd".into()], ", "),
            OsString::from("a, b, cd")
        );
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

    #[test]
    fn check_specialized_harness_name() {
        // It's important that the filenames produced end in `.out` as we produce
        // `--gen-c` filenames with `alter_extension` and we previously had a bug where
        // `for-harness` was the "extension" being removed, and all filenames collided.

        // cargo kani typically produced a file name like this
        let h1 = PathBuf::from("./cbmc-linked.out");
        assert_eq!(specialized_harness_name(&h1, "main"), Path::new("./cbmc-linked.for-main.out"));
        assert_eq!(specialized_harness_name(&h1, "hs_n"), Path::new("./cbmc-linked.for-hs_n.out"));

        // kani typically produces a file name like this
        let h2 = PathBuf::from("./rs-file.out");
        assert_eq!(specialized_harness_name(&h2, "main"), Path::new("./rs-file.for-main.out"));
        assert_eq!(specialized_harness_name(&h2, "hs_n"), Path::new("./rs-file.for-hs_n.out"));
    }
}
