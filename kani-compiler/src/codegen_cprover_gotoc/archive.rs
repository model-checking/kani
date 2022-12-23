// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
// This file is a heavily modified version of upstream rustc:
//     compiler/rustc_codegen_cranelift/src/archive.rs

//! Creation of ar archives like for the lib and staticlib crate type
//! We now call the ArchiveBuilder directly, so we don't bother trying to fit into the rustc's
//! `ArchiveBuilder`.

use rustc_session::Session;
use std::fs::File;
use std::path::{Path, PathBuf};

pub(crate) struct ArchiveBuilder<'a> {
    sess: &'a Session,
    use_gnu_style_archive: bool,

    // Don't use `HashMap` here, as the order is important. `rust.metadata.bin` must always be at
    // the end of an archive for linkers to not get confused.
    entries: Vec<(Vec<u8>, PathBuf)>,
}

impl<'a> ArchiveBuilder<'a> {
    pub fn add_file(&mut self, file: &Path) {
        self.entries.push((
            file.file_name().unwrap().to_str().unwrap().to_string().into_bytes(),
            file.to_owned(),
        ));
    }

    pub fn build(&self, output: &Path) -> bool {
        enum BuilderKind {
            Bsd(ar::Builder<File>),
            Gnu(ar::GnuBuilder<File>),
        }

        let sess = self.sess;

        let mut builder = if self.use_gnu_style_archive {
            BuilderKind::Gnu(ar::GnuBuilder::new(
                File::create(&output).unwrap_or_else(|err| {
                    sess.fatal(&format!(
                        "error opening destination during archive building: {}",
                        err
                    ));
                }),
                self.entries.iter().map(|(name, _)| name.clone()).collect(),
            ))
        } else {
            BuilderKind::Bsd(ar::Builder::new(File::create(&output).unwrap_or_else(|err| {
                sess.fatal(&format!("error opening destination during archive building: {err}"));
            })))
        };

        let entries = self.entries.iter().map(|(entry_name, file)| {
            let data = std::fs::read(file).unwrap_or_else(|err| {
                sess.fatal(&format!(
                    "error while reading object file during archive building: {}",
                    err
                ));
            });
            (entry_name, data)
        });

        // Add all files
        let mut any_members = false;
        for (entry_name, data) in entries {
            let header = ar::Header::new(entry_name.clone(), data.len() as u64);
            match builder {
                BuilderKind::Bsd(ref mut builder) => builder.append(&header, &mut &*data).unwrap(),
                BuilderKind::Gnu(ref mut builder) => builder.append(&header, &mut &*data).unwrap(),
            }
            any_members = true;
        }

        // Finalize archive
        std::mem::drop(builder);
        any_members
    }

    pub fn new(sess: &'a Session) -> Self {
        ArchiveBuilder {
            sess,
            use_gnu_style_archive: sess.target.archive_format == "gnu",
            entries: vec![],
        }
    }
}
