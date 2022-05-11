// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
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

use std::path::Path;
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
