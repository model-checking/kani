// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
// This file is a lightly modified version of upstream rustc:
//     compiler/rustc_codegen_cranelift/src/archive.rs

// Along with lifting the deps:
//  object = { version = "0.27.0", default-features = false, features = ["std", "read_core", "write", "archive", "coff", "elf", "macho", "pe"] }
//  ar = "0.8.0"

// Also: I removed all mentions of 'ranlib' which caused issues on macos.
// We don't actually need these to be passed to a real linker anyway.
// 'ranlib' is about building a global symbol table out of all the object
// files in the archive, and we just don't put object files in our archives.

//! Creation of ar archives like for the lib and staticlib crate type

use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};

use rustc_codegen_ssa::back::archive::{ArchiveBuilder, ArchiveBuilderBuilder};
use rustc_session::Session;

use object::read::archive::ArchiveFile;
use object::ReadCache;

#[derive(Debug)]
enum ArchiveEntry {
    FromArchive { archive_index: usize, file_range: (u64, u64) },
    File(PathBuf),
}

pub(crate) struct ArArchiveBuilder<'a> {
    sess: &'a Session,
    use_gnu_style_archive: bool,

    src_archives: Vec<File>,
    // Don't use `HashMap` here, as the order is important. `rust.metadata.bin` must always be at
    // the end of an archive for linkers to not get confused.
    entries: Vec<(Vec<u8>, ArchiveEntry)>,
}

impl<'a> ArchiveBuilder<'a> for ArArchiveBuilder<'a> {
    fn add_file(&mut self, file: &Path) {
        self.entries.push((
            file.file_name().unwrap().to_str().unwrap().to_string().into_bytes(),
            ArchiveEntry::File(file.to_owned()),
        ));
    }

    fn add_archive(
        &mut self,
        archive_path: &Path,
        mut skip: Box<dyn FnMut(&str) -> bool + 'static>,
    ) -> std::io::Result<()> {
        let read_cache = ReadCache::new(std::fs::File::open(&archive_path)?);
        let archive = ArchiveFile::parse(&read_cache).unwrap();
        let archive_index = self.src_archives.len();

        for entry in archive.members() {
            let entry = entry.map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            let file_name = String::from_utf8(entry.name().to_vec())
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            if !skip(&file_name) {
                self.entries.push((
                    file_name.into_bytes(),
                    ArchiveEntry::FromArchive { archive_index, file_range: entry.file_range() },
                ));
            }
        }

        self.src_archives.push(read_cache.into_inner());
        Ok(())
    }

    fn build(mut self: Box<Self>, output: &Path) -> bool {
        enum BuilderKind {
            Bsd(ar::Builder<File>),
            Gnu(ar::GnuBuilder<File>),
        }

        let sess = self.sess;

        let mut entries = Vec::new();

        for (entry_name, entry) in self.entries {
            // FIXME only read the symbol table of the object files to avoid having to keep all
            // object files in memory at once, or read them twice.
            let data = match entry {
                ArchiveEntry::FromArchive { archive_index, file_range } => {
                    // FIXME read symbols from symtab
                    let src_read_cache = &mut self.src_archives[archive_index];

                    src_read_cache.seek(io::SeekFrom::Start(file_range.0)).unwrap();
                    let mut data = std::vec::from_elem(0, usize::try_from(file_range.1).unwrap());
                    src_read_cache.read_exact(&mut data).unwrap();

                    data
                }
                ArchiveEntry::File(file) => std::fs::read(file).unwrap_or_else(|err| {
                    sess.fatal(&format!(
                        "error while reading object file during archive building: {}",
                        err
                    ));
                }),
            };

            entries.push((entry_name, data));
        }

        let mut builder = if self.use_gnu_style_archive {
            BuilderKind::Gnu(ar::GnuBuilder::new(
                File::create(&output).unwrap_or_else(|err| {
                    sess.fatal(&format!(
                        "error opening destination during archive building: {}",
                        err
                    ));
                }),
                entries.iter().map(|(name, _)| name.clone()).collect(),
            ))
        } else {
            BuilderKind::Bsd(ar::Builder::new(File::create(&output).unwrap_or_else(|err| {
                sess.fatal(&format!("error opening destination during archive building: {err}"));
            })))
        };

        // Add all files
        let any_members = !entries.is_empty();
        for (entry_name, data) in entries.into_iter() {
            let header = ar::Header::new(entry_name, data.len() as u64);
            match builder {
                BuilderKind::Bsd(ref mut builder) => builder.append(&header, &mut &*data).unwrap(),
                BuilderKind::Gnu(ref mut builder) => builder.append(&header, &mut &*data).unwrap(),
            }
        }

        // Finalize archive
        std::mem::drop(builder);
        any_members
    }
}

pub(crate) struct ArArchiveBuilderBuilder;

impl ArchiveBuilderBuilder for ArArchiveBuilderBuilder {
    fn new_archive_builder<'a>(&self, sess: &'a Session) -> Box<dyn ArchiveBuilder<'a> + 'a> {
        Box::new(ArArchiveBuilder {
            sess,
            use_gnu_style_archive: sess.target.archive_format == "gnu",
            src_archives: vec![],
            entries: vec![],
        })
    }

    fn create_dll_import_lib(
        &self,
        _sess: &Session,
        _lib_name: &str,
        _dll_imports: &[rustc_session::cstore::DllImport],
        _tmpdir: &Path,
        _is_direct_dependency: bool,
    ) -> PathBuf {
        unimplemented!("injecting dll imports is not supported");
    }
}
