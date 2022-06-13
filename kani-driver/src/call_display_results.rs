// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::session::KaniSession;
use crate::util::render_command;

impl KaniSession {
    /// Display the results of a CBMC run in a user-friendly manner.
    pub fn format_cbmc_output(&self, file: &Path) -> Result<()> {
        let mut args: Vec<OsString> = vec![
            self.cbmc_json_parser_py.clone().into(),
            file.into(),
            self.args.output_format.to_string().to_lowercase().into(),
        ];

        if self.args.extra_pointer_checks {
            args.push("--extra-ptr-check".into());
        }

        let mut cmd = Command::new("python3");
        cmd.args(args);

        println!("{}", render_command(&cmd).to_string_lossy());

        self.run_terminal(cmd)?;

        Ok(())
    }

    /// Display the results of a CBMC run in a user-friendly manner.
    pub fn format_cbmc_output_live(&self) -> Result<()> {
        let mut args: Vec<OsString> = vec![
            self.cbmc_json_parser_py.clone().into(),
            "read_from_pipe".into(),
            self.args.output_format.to_string().to_lowercase().into(),
        ];

        if self.args.extra_pointer_checks {
            args.push("--extra-ptr-check".into());
        }

        let mut cmd = Command::new("python3");
        cmd.args(args);

        println!("{}", render_command(&cmd).to_string_lossy());

        self.run_terminal(cmd)?;

        Ok(())
    }
}
