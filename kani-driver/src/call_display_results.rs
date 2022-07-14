// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::session::KaniSession;

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

        self.run_terminal(cmd)?;

        Ok(())
    }

    /// Display the results of a CBMC run in a user-friendly manner.
    pub fn format_cbmc_output_live(&self) -> Result<Command> {
        // Add flag --read-cbmc-from-stream for the parser
        let mut python_args: Vec<OsString> = vec![
            self.cbmc_json_parser_py.clone().into(),
            "--read-cbmc-from-stream".into(),
            self.args.output_format.to_string().to_lowercase().into(),
        ];

        if self.args.extra_pointer_checks {
            python_args.push("--extra-ptr-check".into());
        }

        // This is the equivalent to running the command
        // > cbmc --flags test_file.out.for-main --json | python --read-cbmc-from-stream regular
        let mut python_command = Command::new("python3");
        python_command.args(python_args);

        Ok(python_command)
    }
}
