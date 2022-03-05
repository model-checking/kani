// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Replace an extension with another one, in a new PathBuf. (See tests for examples)
pub fn alter_extension(path: &Path, ext: &str) -> PathBuf {
    let mut result = path.to_owned();
    result.set_extension(ext);
    result
}

/// Add an extension to an existing file path (amazingly Rust doesn't support this well)
pub fn append_path(path: &Path, ext: &str) -> PathBuf {
    let mut str = path.to_owned().into_os_string();
    str.push(".");
    str.push(ext);
    str.into()
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
    fn check_append_path() {
        assert_eq!(append_path(&PathBuf::from("./file"), "tar"), PathBuf::from("./file.tar"));
        assert_eq!(
            append_path(&PathBuf::from("./file.symtab.json"), "out"),
            PathBuf::from("./file.symtab.json.out")
        );
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
}
