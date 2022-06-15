// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::metadata::collect_and_link_function_pointer_restrictions;
use crate::session::KaniSession;
use crate::util::alter_extension;

impl KaniSession {
    /// Postprocess a goto binary (before cbmc, after linking) in-place by calling goto-instrument
    pub fn run_goto_instrument(&self, input: &Path, output: &Path, function: &str) -> Result<()> {
        // We actually start by calling goto-cc to start the specialization:
        self.specialize_to_proof_harness(input, output, function)?;

        if self.args.checks.undefined_function_on() {
            self.add_library(output)?;
            self.undefined_functions(output)?;
        } else {
            self.just_drop_unused_functions(output)?;
        }

        self.rewrite_back_edges(output)?;

        if self.args.gen_c {
            if !self.args.quiet {
                println!(
                    "Generated C code written to {}",
                    alter_extension(output, "c").to_string_lossy()
                );
            }
            self.gen_c(output)?;
        }

        Ok(())
    }

    /// Apply --restrict-vtable to a goto binary.
    /// `source` is either a `*.restrictions.json` file or a directory containing mutiple such files.
    pub fn apply_vtable_restrictions(&self, file: &Path, source: &Path) -> Result<()> {
        let linked_restrictions = alter_extension(file, "linked-restrictions.json");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(linked_restrictions.clone());
        }

        collect_and_link_function_pointer_restrictions(source, &linked_restrictions)?;

        let args: Vec<OsString> = vec![
            "--function-pointer-restrictions-file".into(),
            linked_restrictions.into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        self.call_goto_instrument(args)
    }

    /// Link the binary against the CBMC model for C library functions.
    /// Normally this happens implicitly, but we use this explicitly
    /// before we invoke `undefined_functions` below, otherwise these
    /// functions appear undefined.
    fn add_library(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--add-library".into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        self.call_goto_instrument(args)
    }

    /// Instruct CBMC to "assert false" when invoking an undefined function.
    /// (This contrasts with its default behavior of returning `nondet`, which is
    /// unsound in the face of side-effects.)
    /// Then remove unused functions. (Oddly, it seems CBMC will both see some
    /// functions as unused and remove them, and also as used and so would
    /// generate "assert false". So it's essential to do this afterwards.)
    fn undefined_functions(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--generate-function-body-options".into(),
            "assert-false-assume-false".into(),
            "--generate-function-body".into(),
            ".*".into(),
            //"--drop-unused-functions".into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        self.call_goto_instrument(args)
    }

    /// Remove all functions unreachable from the current proof harness.
    fn just_drop_unused_functions(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--drop-unused-functions".into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        self.call_goto_instrument(args)
    }

    fn rewrite_back_edges(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--ensure-one-backedge-per-target".into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        self.call_goto_instrument(args)
    }

    /// Generate a .c file from a goto binary (i.e. --gen-c)
    pub fn gen_c(&self, file: &Path) -> Result<()> {
        let output_filename = alter_extension(file, "c");
        // We don't put the C file into temporaries to be deleted.

        let args: Vec<OsString> = vec![
            "--dump-c".into(),
            file.to_owned().into_os_string(),
            output_filename.into_os_string(),
        ];

        self.call_goto_instrument(args)
    }

    /// Non-public helper function to actually do the run of goto-instrument
    fn call_goto_instrument(&self, args: Vec<OsString>) -> Result<()> {
        // TODO get goto-instrument path from self
        let mut cmd = Command::new("goto-instrument");
        cmd.args(args);

        self.run_suppress(cmd)
    }
}
