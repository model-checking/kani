//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::any::Any;
use core::fmt::Debug;
use core::num::ParseIntError;
use std::borrow::{Cow, ToOwned};
use std::boxed::Box;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::vec::Vec;
use test_runner::failure_persistence::FailurePersistence;

/// Describes how failing test cases are persisted.
///
/// Note that file names in this enum are `&str` rather than `&Path` since
/// constant functions are not yet in Rust stable as of 2017-12-16.
///
/// In all cases, if a derived path references a directory which does not yet
/// exist, proptest will attempt to create all necessary parent directories.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FileFailurePersistence {
    /// Completely disables persistence of failing test cases.
    ///
    /// This is semantically equivalent to `Direct("/dev/null")` on Unix and
    /// `Direct("NUL")` on Windows (though it is internally handled by simply
    /// not doing any I/O).
    Off,
    /// The path given to `TestRunner::set_source_file()` is parsed. The path
    /// is traversed up the directory tree until a directory containing a file
    /// named `lib.rs` or `main.rs` is found. A sibling to that directory with
    /// the name given by the string in this configuration is created, and a
    /// file with the same name and path relative to the source directory, but
    /// with the extension changed to `.txt`, is used.
    ///
    /// For example, given a source path of
    /// `/home/jsmith/code/project/src/foo/bar.rs` and a configuration of
    /// `SourceParallel("proptest-regressions")` (the default), assuming the
    /// `src` directory has a `lib.rs` or `main.rs`, the resulting file would
    /// be `/home/jsmith/code/project/proptest-regressions/foo/bar.txt`.
    ///
    /// If no `lib.rs` or `main.rs` can be found, a warning is printed and this
    /// behaves like `WithSource`.
    ///
    /// If no source file has been configured, a warning is printed and this
    /// behaves like `Off`.
    SourceParallel(&'static str),
    /// The path given to `TestRunner::set_source_file()` is parsed. The
    /// extension of the path is changed to the string given in this
    /// configuration, and that filename is used.
    ///
    /// For example, given a source path of
    /// `/home/jsmith/code/project/src/foo/bar.rs` and a configuration of
    /// `WithSource("regressions")`, the resulting path would be
    /// `/home/jsmith/code/project/src/foo/bar.regressions`.
    WithSource(&'static str),
    /// The string given in this option is directly used as a file path without
    /// any further processing.
    Direct(&'static str),
    #[doc(hidden)]
    #[allow(missing_docs)]
    _NonExhaustive,
}

/// Ensure that the source file to use for resolving the location of the persisted
/// failing cases file is absolute.
///
/// The source location can only be used if it is absolute. If `source` is
/// not an absolute path, an attempt will be made to determine the absolute
/// path based on the current working directory and its parents. If no
/// absolute path can be determined, a warning will be printed and proptest
/// will continue as if this function had never been called.
///
/// See [`FileFailurePersistence`](enum.FileFailurePersistence.html) for details on
/// how this value is used once it is made absolute.
///
/// This is normally called automatically by the `proptest!` macro, which
/// passes `file!()`.
///
fn absolutize_source_file(source: &'static Path) -> Option<Cow<'static, Path>> {
    absolutize_source_file_with_cwd(env::current_dir, source)
}

fn absolutize_source_file_with_cwd<F>(
    getcwd: F,
    source: &'static Path,
) -> Option<Cow<'static, Path>>
where
    F: FnOnce() -> io::Result<PathBuf>,
{
    if source.is_absolute() {
        // On Unix, `file!()` is absolute. In these cases, we can use
        // that path directly.
        Some(Cow::Borrowed(source))
    } else {
        // On Windows, `file!()` is relative to the crate root, but the
        // test is not generally run with the crate root as the working
        // directory, so the path is not directly usable. However, the
        // working directory is almost always a subdirectory of the crate
        // root, so pop directories off until pushing the source onto the
        // directory results in a path that refers to an existing file.
        // Once we find such a path, we can use that.
        //
        // If we can't figure out an absolute path, print a warning and act
        // as if no source had been given.
        match getcwd() {
            Ok(mut cwd) => loop {
                let joined = cwd.join(source);
                if joined.is_file() {
                    break Some(Cow::Owned(joined));
                }

                if !cwd.pop() {
                    eprintln!(
                        "proptest: Failed to find absolute path of \
                         source file '{:?}'. Ensure the test is \
                         being run from somewhere within the crate \
                         directory hierarchy.",
                        source
                    );
                    break None;
                }
            },

            Err(e) => {
                eprintln!(
                    "proptest: Failed to determine current \
                     directory, so the relative source path \
                     '{:?}' cannot be resolved: {}",
                    source, e
                );
                None
            }
        }
    }
}

impl FailurePersistence for FileFailurePersistence {
    fn load_persisted_failures(&self, source_file: Option<&'static str>) -> Vec<[u32; 4]> {
        let p = self.resolve(
                source_file
                    .and_then(|s| absolutize_source_file(Path::new(s)))
                    .as_ref()
                    .map(|cow| &**cow));

        let path: Option<&PathBuf> = match p {
            Some(ref inner_path) => Some(inner_path),
            None => None,
        };
        let result: io::Result<Vec<[u32; 4]>> = path.map_or_else(
            || Ok(vec![]),
            |path| {
                // .ok() instead of .unwrap() so we don't propagate panics here
                let _lock = PERSISTENCE_LOCK.read().ok();

                let mut ret = Vec::new();

                let input = io::BufReader::new(fs::File::open(path)?);
                for (lineno, line) in input.lines().enumerate() {
                    let mut line = line?;
                    if let Some(comment_start) = line.find('#') {
                        line.truncate(comment_start);
                    }

                    let parts = line.trim().split(' ').collect::<Vec<_>>();
                    if 5 == parts.len() && "xs" == parts[0] {
                        if let Ok(seed) = parse_seed(&*parts) {
                            ret.push(seed);
                        } else {
                            eprintln!(
                                "proptest: {}:{}: unparsable line, \
                             ignoring",
                                path.display(),
                                lineno + 1
                            );
                        }
                    } else if parts.len() > 1 {
                        eprintln!(
                            "proptest: {}:{}: unknown case type `{}` \
                         (corrupt file or newer proptest version?)",
                            &path.display(),
                            lineno + 1,
                            parts[0]
                        );
                    }
                }

                Ok(ret)
            },
        );

        match result {
            Ok(r) => r,
            Err(err) => {
                if io::ErrorKind::NotFound != err.kind() {
                    eprintln!(
                        "proptest: failed to open {}: {}",
                        &path.map(|x| &**x)
                            .unwrap_or_else(|| Path::new("??"))
                            .display(),
                        err
                    );
                }
                vec![]
            }
        }
    }


    fn save_persisted_failure(
        &mut self,
        source_file: Option<&'static str>,
        seed: [u32; 4],
        shrunken_value: &Debug,
    ) {
        let path = self.resolve(source_file.map(|f| Path::new(f)));
        if let Some(path) = path {
            // .ok() instead of .unwrap() so we don't propagate panics here
            let _lock = PERSISTENCE_LOCK.write().ok();
            let is_new = !path.is_file();

            let mut to_write = Vec::<u8>::new();
            if is_new {
                writeln!(
                    to_write,
                    "\
# Seeds for failure cases proptest has generated in the past. It is
# automatically read and these particular cases re-run before any
# novel cases are generated.
#
# It is recommended to check this file in to source control so that
# everyone who runs the test benefits from these saved cases."
                ).expect("writeln! to vec failed");
            }
            let mut data_line = Vec::<u8>::new();
            write!(
                data_line,
                "xs {} {} {} {} # shrinks to {:?}",
                seed[0], seed[1], seed[2], seed[3], shrunken_value
            ).expect("write! to vec failed");
            // Ensure there are no newlines in the debug output
            for byte in &mut data_line {
                if b'\n' == *byte || b'\r' == *byte {
                    *byte = b' ';
                }
            }
            to_write.extend(data_line);
            to_write.push(b'\n');

            fn do_write(dst: &Path, data: &[u8]) -> io::Result<()> {
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }

                let mut options = fs::OpenOptions::new();
                options.append(true).create(true);
                let mut out = options.open(dst)?;
                out.write_all(data)?;

                Ok(())
            }

            if let Err(e) = do_write(&path, &to_write) {
                eprintln!("proptest: failed to append to {}: {}", path.display(), e);
            } else if is_new {
                eprintln!(
                    "proptest: Saving this and future failures in {}",
                    path.display()
                );
            }
        }
    }

    fn box_clone(&self) -> Box<FailurePersistence> {
        Box::new(self.clone())
    }

    fn eq(&self, other: &FailurePersistence) -> bool {
        other.as_any().downcast_ref::<Self>().map_or(false, |x| x == self)
    }

    fn as_any(&self) -> &Any { self }
}

use self::FileFailurePersistence::*;

impl Default for FileFailurePersistence {
    fn default() -> Self {
        SourceParallel("proptest-regressions")
    }
}

impl FileFailurePersistence {
    /// Given the nominal source path, determine the location of the failure
    /// persistence file, if any.
    pub(super) fn resolve(&self, source: Option<&Path>) -> Option<PathBuf> {
        match *self {
            Off => None,

            SourceParallel(sibling) => match source {
                Some(source_path) => {
                    let mut dir = source_path.to_owned();
                    let mut found = false;
                    while dir.pop() {
                        if dir.join("lib.rs").is_file() || dir.join("main.rs").is_file() {
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        eprintln!(
                            "proptest: FileFailurePersistence::SourceParallel set, \
                             but failed to find lib.rs or main.rs"
                        );
                        WithSource(sibling).resolve(source)
                    } else {
                        let suffix = source_path
                            .strip_prefix(&dir)
                            .expect("parent of source is not a prefix of it?")
                            .to_owned();
                        let mut result = dir;
                        // If we've somehow reached the root, or someone gave
                        // us a relative path that we've exhausted, just accept
                        // creating a subdirectory instead.
                        let _ = result.pop();
                        result.push(sibling);
                        result.push(&suffix);
                        result.set_extension("txt");
                        Some(result)
                    }
                }
                None => {
                    eprintln!(
                        "proptest: FileFailurePersistence::SourceParallel set, \
                         but no source file known"
                    );
                    None
                }
            },

            WithSource(extension) => match source {
                Some(source_path) => {
                    let mut result = source_path.to_owned();
                    result.set_extension(extension);
                    Some(result)
                }

                None => {
                    eprintln!(
                        "proptest: FileFailurePersistence::WithSource set, \
                         but no source file known"
                    );
                    None
                }
            },

            Direct(path) => Some(Path::new(path).to_owned()),

            _NonExhaustive => panic!("FailurePersistence set to _NonExhaustive"),
        }
    }
}

lazy_static! {
    /// Used to guard access to the persistence file(s) so that a single
    /// process will not step on its own toes.
    ///
    /// We don't have much protecting us should two separate process try to
    /// write to the same file at once (depending on how atomic append mode is
    /// on the OS), but this should be extremely rare.
    static ref PERSISTENCE_LOCK: RwLock<()> = RwLock::new(());
}



fn parse_seed(parts: &[&str]) -> Result<[u32; 4], ParseIntError> {
    let a = parts[1].parse()?;
    let b = parts[2].parse()?;
    let c = parts[3].parse()?;
    let d = parts[4].parse()?;
    Ok([a, b, c, d])
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPaths {
        crate_root: &'static Path,
        src_file: PathBuf,
        subdir_file: PathBuf,
        misplaced_file: PathBuf,
    }

    lazy_static! {
        static ref TEST_PATHS: TestPaths = {
            let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
            let lib_root = crate_root.join("src");
            let src_subdir = lib_root.join("strategy");
            let src_file = lib_root.join("foo.rs");
            let subdir_file = src_subdir.join("foo.rs");
            let misplaced_file = crate_root.join("foo.rs");
            TestPaths {
                crate_root,
                src_file,
                subdir_file,
                misplaced_file,
            }
        };
    }

    #[test]
    fn persistence_file_location_resolved_correctly() {
        // If off, there is never a file
        assert_eq!(None, Off.resolve(None));
        assert_eq!(None, Off.resolve(Some(&TEST_PATHS.subdir_file)));

        // For direct, we don't care about the source file, and instead always
        // use whatever is in the config.
        assert_eq!(
            Some(Path::new("bar.txt").to_owned()),
            Direct("bar.txt").resolve(None)
        );
        assert_eq!(
            Some(Path::new("bar.txt").to_owned()),
            Direct("bar.txt").resolve(Some(&TEST_PATHS.subdir_file))
        );

        // For WithSource, only the extension changes, but we get nothing if no
        // source file was configured.
        // Accounting for the way absolute paths work on Windows would be more
        // complex, so for now don't test that case.
        #[cfg(unix)]
        fn absolute_path_case() {
            assert_eq!(
                Some(Path::new("/foo/bar.ext").to_owned()),
                WithSource("ext").resolve(Some(Path::new("/foo/bar.rs")))
            );
        }
        #[cfg(not(unix))]
        fn absolute_path_case() {}
        absolute_path_case();
        assert_eq!(None, WithSource("ext").resolve(None));

        // For SourceParallel, we make a sibling directory tree and change the
        // extensions to .txt ...
        assert_eq!(
            Some(TEST_PATHS.crate_root.join("sib").join("foo.txt")),
            SourceParallel("sib").resolve(Some(&TEST_PATHS.src_file))
        );
        assert_eq!(
            Some(
                TEST_PATHS
                    .crate_root
                    .join("sib")
                    .join("strategy")
                    .join("foo.txt")
            ),
            SourceParallel("sib").resolve(Some(&TEST_PATHS.subdir_file))
        );
        // ... but if we can't find lib.rs / main.rs, give up and set the
        // extension instead ...
        assert_eq!(
            Some(TEST_PATHS.crate_root.join("foo.sib")),
            SourceParallel("sib").resolve(Some(&TEST_PATHS.misplaced_file))
        );
        // ... and if no source is configured, we do nothing
        assert_eq!(None, SourceParallel("ext").resolve(None));
    }

    #[test]
    fn relative_source_files_absolutified() {
        const TEST_RUNNER_PATH: &[&str] = &["src", "test_runner", "mod.rs"];
        lazy_static! {
            static ref TEST_RUNNER_RELATIVE: PathBuf = TEST_RUNNER_PATH.iter().collect();
        }
        const CARGO_DIR: &str = env!("CARGO_MANIFEST_DIR");

        let expected = ::std::iter::once(CARGO_DIR)
            .chain(TEST_RUNNER_PATH.iter().map(|s| *s))
            .collect::<PathBuf>();

        // Running from crate root
        assert_eq!(
            &*expected,
            absolutize_source_file_with_cwd(
                || Ok(Path::new(CARGO_DIR).to_owned()),
                &TEST_RUNNER_RELATIVE
            ).unwrap()
        );

        // Running from test subdirectory
        assert_eq!(
            &*expected,
            absolutize_source_file_with_cwd(
                || Ok(Path::new(CARGO_DIR).join("target")),
                &TEST_RUNNER_RELATIVE
            ).unwrap()
        );
    }
}
