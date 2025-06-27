// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains small helper functions for running Commands.
//! We could possibly eliminate this if we find a small-enough dependency.

use std::ffi::OsString;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Helper trait to fallibly run commands
pub trait AutoRun {
    fn run(&mut self) -> Result<()>;
}
impl AutoRun for Command {
    fn run(&mut self) -> Result<()> {
        // This can sometimes fail during the set-up of the forked process before exec,
        // for example by setting `current_dir` to a directory that does not exist.
        let status = self.status().with_context(|| {
            format!(
                "Internal failure before invoking command: {}",
                render_command(self).to_string_lossy()
            )
        })?;
        if !status.success() {
            bail!("Failed command: {}", render_command(self).to_string_lossy());
        }
        Ok(())
    }
}

/// Render a Command as a string, to log it
fn render_command(cmd: &Command) -> OsString {
    let mut str = OsString::new();

    for (k, v) in cmd.get_envs() {
        if let Some(v) = v {
            str.push(k);
            str.push("=\"");
            str.push(v);
            str.push("\" ");
        }
    }

    str.push(cmd.get_program());

    for a in cmd.get_args() {
        str.push(" ");
        if a.to_string_lossy().contains(' ') {
            str.push("\"");
            str.push(a);
            str.push("\"");
        } else {
            str.push(a);
        }
    }

    str
}
