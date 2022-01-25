//! Rustdoc's FileSystem abstraction module.
//!
//! On Windows this indirects IO into threads to work around performance issues
//! with Defender (and other similar virus scanners that do blocking operations).
//! On other platforms this is a thin shim to fs.
//!
//! Only calls needed to permit this workaround have been abstracted: thus
//! fs::read is still done directly via the fs module; if in future rustdoc
//! needs to read-after-write from a file, then it would be added to this
//! abstraction.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::string::ToString;
use std::sync::mpsc::Sender;

crate trait PathError {
    fn new<S, P: AsRef<Path>>(e: S, path: P) -> Self
    where
        S: ToString + Sized;
}

crate struct DocFS {
    sync_only: bool,
    errors: Option<Sender<String>>,
}

impl DocFS {
    crate fn new(errors: Sender<String>) -> DocFS {
        DocFS { sync_only: false, errors: Some(errors) }
    }

    crate fn set_sync_only(&mut self, sync_only: bool) {
        self.sync_only = sync_only;
    }

    crate fn close(&mut self) {
        self.errors = None;
    }

    crate fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        // For now, dir creation isn't a huge time consideration, do it
        // synchronously, which avoids needing ordering between write() actions
        // and directory creation.
        fs::create_dir_all(path)
    }

    crate fn write<E>(
        &self,
        path: PathBuf,
        contents: impl 'static + Send + AsRef<[u8]>,
    ) -> Result<(), E>
    where
        E: PathError,
    {
        if !self.sync_only && cfg!(windows) {
            // A possible future enhancement after more detailed profiling would
            // be to create the file sync so errors are reported eagerly.
            let sender = self.errors.clone().expect("can't write after closing");
            rayon::spawn(move || {
                fs::write(&path, contents).unwrap_or_else(|e| {
                    sender.send(format!("\"{}\": {}", path.display(), e)).unwrap_or_else(|_| {
                        panic!("failed to send error on \"{}\"", path.display())
                    })
                });
            });
        } else {
            fs::write(&path, contents).map_err(|e| E::new(e, path))?;
        }
        Ok(())
    }
}
