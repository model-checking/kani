// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::context::RmcContext;
use crate::util::alter_extension;

impl RmcContext {
    pub fn compile_single_rust_file(&self, file: &Path) -> Result<PathBuf> {
        let output_filename = alter_extension(file, "symtab.json");

        {
            let type_map_filename = alter_extension(file, "type_map.json");
            let metadata_filename = alter_extension(file, "rmc-metadata.json");
            let mut temps = self.temporaries.borrow_mut();
            temps.push(output_filename.clone());
            temps.push(type_map_filename);
            temps.push(metadata_filename);
        }

        let mut args = self.rmc_rustc_flags();
        args.push(file.to_owned().into_os_string());

        let result = Command::new(&self.rmc_rustc)
            .args(args)
            .status()
            .context("Failed to invoke rmc-rustc")?;

        if !result.success() {
            bail!("rmc-rustc exited with status {}", result);
        }

        Ok(output_filename)
    }

    pub fn rmc_rustc_flags(&self) -> Vec<OsString> {
        let flags = vec![
            "--cfg=rmc", // not actually necessary it's in rmc-rustc
                         //"-Z", "human_readable_cgu_names",
                         //"-Z", "symbol-mangling-version=v0", // todo
        ];
        flags.iter().map(|x| x.into()).collect()
    }
}
