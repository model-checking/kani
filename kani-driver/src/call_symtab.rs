// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::KaniSession;

impl KaniSession {
    /// Given a `file.symtab.json`, produce `{file}.symtab.out` by calling symtab2gb
    pub fn symbol_table_to_gotoc(&self, file: &Path) -> Result<PathBuf> {
        let output_filename = file.with_extension("out");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(output_filename.clone());
        }

        let args = vec![
            file.to_owned().into_os_string(),
            "--out".into(),
            output_filename.clone().into_os_string(),
        ];
        // TODO get symtab2gb path from self
        let mut cmd = Command::new("symtab2gb");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(output_filename)
    }
}
