// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// See GitHub history for details.

use crate::common::Config;
use std::ffi::OsStr;
use std::path::PathBuf;

use tracing::*;

pub fn logv(config: &Config, s: String) {
    debug!("{}", s);
    if config.verbose {
        println!("{}", s);
    }
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
