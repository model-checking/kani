// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::context::KaniContext;
use crate::util::alter_extension;

impl KaniContext {
    pub fn compile_single_rust_file(&self, file: &Path) -> Result<PathBuf> {
        let output_filename = alter_extension(file, "symtab.json");

        {
            let type_map_filename = alter_extension(file, "type_map.json");
            let metadata_filename = alter_extension(file, "kani-metadata.json");
            let mut temps = self.temporaries.borrow_mut();
            temps.push(output_filename.clone());
            temps.push(type_map_filename);
            temps.push(metadata_filename);
        }

        let mut args = self.kani_rustc_flags();
        args.push(file.to_owned().into_os_string());

        let result = Command::new(&self.kani_rustc)
            .args(args)
            .status()
            .context("Failed to invoke kani-rustc")?;

        if !result.success() {
            bail!("kani-rustc exited with status {}", result);
        }

        Ok(output_filename)
    }

    pub fn kani_rustc_flags(&self) -> Vec<OsString> {
        let flags = vec!["--goto-c"];
        flags.iter().map(|x| x.into()).collect()
    }
}
