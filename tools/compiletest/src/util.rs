// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

use crate::common::Config;
use std::ffi::OsStr;
use std::path::PathBuf;

use std::process::Command;
use tracing::*;

pub fn logv(config: &Config, s: String) {
    debug!("{}", s);
    if config.verbose {
        println!("{s}");
    }
}

/// Print a message as long as we are not running under --quiet. In quiet mode, we log the message.
pub fn print_msg(config: &Config, msg: String) {
    if config.quiet { debug!("{msg}") } else { println!("{msg}") }
}

pub trait PathBufExt {
    /// Append an extension to the path, even if it already has one.
    fn with_extra_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf;
}

impl PathBufExt for PathBuf {
    fn with_extra_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        if extension.as_ref().is_empty() {
            self.clone()
        } else {
            let mut fname = self.file_name().unwrap().to_os_string();
            if !extension.as_ref().to_str().unwrap().starts_with('.') {
                fname.push(".");
            }
            fname.push(extension);
            self.with_file_name(fname)
        }
    }
}

pub(crate) fn top_level() -> Option<PathBuf> {
    match Command::new("git").arg("rev-parse").arg("--show-toplevel").output() {
        Ok(out) if out.status.success() => {
            Some(PathBuf::from(String::from_utf8(out.stdout).unwrap().trim()))
        }
        _ => None,
    }
}
