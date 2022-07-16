// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;

use crate::metadata::collect_and_link_function_pointer_restrictions;
use crate::session::KaniSession;
use crate::util::alter_extension;

impl KaniSession {
    /// Postprocess a goto binary (before cbmc, after linking) in-place by calling goto-instrument
    pub fn run_goto_instrument(
        &self,
        input: &Path,
        output: &Path,
        symtabs: &[impl AsRef<Path>],
        function: &str,
    ) -> Result<()> {
        // We actually start by calling goto-cc to start the specialization:
        self.specialize_to_proof_harness(input, output, function)?;

        if self.args.checks.undefined_function_on() {
            self.add_library(output)?;
            self.undefined_functions(output)?;
        } else {
            self.just_drop_unused_functions(output)?;
        }

        self.rewrite_back_edges(output)?;

        if self.args.run_sanity_checks {
            self.goto_sanity_check(output)?;
        }

        if self.args.gen_c {
            let c_outfile = alter_extension(output, "c");
            // We don't put the C file into temporaries to be deleted.

            self.gen_c(output, &c_outfile)?;

            if !self.args.quiet {
                println!("Generated C code written to {}", c_outfile.to_string_lossy());
            }

            let c_demangled = alter_extension(output, "demangled.c");
            self.demangle_c(symtabs, &c_outfile, &c_demangled)?;
            if !self.args.quiet {
                println!("Demangled GotoC code written to {}", c_demangled.to_string_lossy())
            }
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
            "--drop-unused-functions".into(),
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

    fn goto_sanity_check(&self, file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--validate-goto-model".into(),
            file.to_owned().into_os_string(), // input
            file.to_owned().into_os_string(), // output
        ];

        self.call_goto_instrument(args)
    }

    /// Generate a .c file from a goto binary (i.e. --gen-c)
    pub fn gen_c(&self, file: &Path, output_file: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--dump-c".into(),
            file.to_owned().into_os_string(),
            output_file.to_owned().into_os_string(),
        ];

        self.call_goto_instrument(args)
    }

    /// Generate a .demangled.c file from the .c file using the `prettyName`s from the symbol tables
    ///
    /// Currently, only top-level function names and (most) type names are demangled.
    /// For local variables, it would be more complicated than a simple search and replace to obtain the demangled name.
    pub fn demangle_c(
        &self,
        symtab_files: &[impl AsRef<Path>],
        c_file: &Path,
        demangled_file: &Path,
    ) -> Result<()> {
        let mut c_code = std::fs::read_to_string(c_file)?;
        for symtab_file in symtab_files {
            let reader = BufReader::new(File::open(symtab_file.as_ref())?);
            let symtab: serde_json::Value = serde_json::from_reader(reader)?;
            for (_, symbol) in symtab["symbolTable"].as_object().unwrap() {
                if let Some(serde_json::Value::String(name)) = symbol.get("name") {
                    if let Some(serde_json::Value::String(pretty)) = symbol.get("prettyName") {
                        // Struct names start with "tag-", but this prefix is not used in the GotoC files, so we strip it.
                        // If there is no such prefix, we leave the name unchanged.
                        let name = name.strip_prefix("tag-").unwrap_or(name);
                        if !pretty.is_empty() && pretty != name {
                            c_code = c_code.replace(name, pretty);
                        }
                    }
                }
            }
        }
        std::fs::write(demangled_file, c_code)?;
        Ok(())
    }

    /// Non-public helper function to actually do the run of goto-instrument
    fn call_goto_instrument(&self, args: Vec<OsString>) -> Result<()> {
        // TODO get goto-instrument path from self
        let mut cmd = Command::new("goto-instrument");
        cmd.args(args);

        self.run_suppress(cmd)
    }
}
