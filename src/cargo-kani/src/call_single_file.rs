// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
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

        if self.args.tests {
            args.push("--test".into());
        }

        let mut cmd = Command::new(&self.kani_rustc);
        cmd.args(args);

        if self.args.debug && !self.args.quiet {
            cmd.env("KANI_LOG", "rustc_codegen_kani");
            self.run_terminal(cmd)?;
        } else {
            self.run_suppress(cmd)?;
        }

        Ok(output_filename)
    }

    /// These arguments are passed directly here for single file runs,
    /// but are also used by call_cargo to pass as the env var KaniFLAGS.
    pub fn kani_rustc_flags(&self) -> Vec<OsString> {
        let mut flags = vec!["--goto-c".to_string()];

        if self.args.use_abs {
            flags.push("-Z".into());
            flags.push("force-unstable-if-unmarked=yes".into()); // ??
            flags.push("--cfg=use_abs".into());
            flags.push("--cfg".into());
            flags.push(format!("abs_type={}", self.args.abs_type.to_string().to_lowercase()));
        }

        flags.iter().map(|x| x.into()).collect()
    }
}
