// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;

use crate::metadata::collect_and_link_function_pointer_restrictions;
use crate::project::Project;
use crate::session::KaniSession;
use crate::util::alter_extension;
use kani_metadata::{ArtifactType, HarnessMetadata};

impl KaniSession {
    /// Instrument and optimize a goto binary in-place.
    pub fn instrument_model(
        &self,
        input: &Path,
        output: &Path,
        project: &Project,
        harness: &HarnessMetadata,
    ) -> Result<()> {
        // We actually start by calling goto-cc to start the specialization:
        self.specialize_to_proof_harness(input, output, &harness.mangled_name)?;

        let restrictions = project.get_harness_artifact(harness, ArtifactType::VTableRestriction);
        if let Some(restrictions_path) = restrictions {
            self.apply_vtable_restrictions(output, restrictions_path)?;
        }

        // Run sanity checks in the model generated by kani-compiler before any goto-instrument
        // transformation.
        if self.args.run_sanity_checks {
            self.goto_sanity_check(output)?;
        }

        let is_loop_contracts_enabled = self
            .args
            .common_args
            .unstable_features
            .contains(kani_metadata::UnstableFeature::LoopContracts)
            && harness.has_loop_contracts;
        self.instrument_contracts(harness, is_loop_contracts_enabled, output)?;

        if self.args.checks.undefined_function_on() {
            self.add_library(output)?;
            self.undefined_functions(output)?;
        } else {
            self.just_drop_unused_functions(output)?;
        }

        self.rewrite_back_edges(output)?;

        if self.args.gen_c {
            let c_outfile = alter_extension(output, "c");
            // We don't put the C file into temporaries to be deleted.

            self.gen_c(output, &c_outfile)?;

            if !self.args.common_args.quiet {
                println!("Generated C code written to {}", c_outfile.to_string_lossy());
            }

            let c_demangled = alter_extension(output, "demangled.c");
            let prett_name_map =
                project.get_harness_artifact(harness, ArtifactType::PrettyNameMap).unwrap();
            self.demangle_c(prett_name_map, &c_outfile, &c_demangled)?;
            if !self.args.common_args.quiet {
                println!("Demangled GotoC code written to {}", c_demangled.to_string_lossy())
            }
        }

        Ok(())
    }

    /// Apply -Z restrict-vtable to a goto binary.
    pub fn apply_vtable_restrictions(&self, goto_file: &Path, restrictions: &Path) -> Result<()> {
        let linked_restrictions = alter_extension(goto_file, "linked-restrictions.json");
        self.record_temporary_file(&linked_restrictions);
        collect_and_link_function_pointer_restrictions(restrictions, &linked_restrictions)?;

        let args: Vec<OsString> = vec![
            "--function-pointer-restrictions-file".into(),
            linked_restrictions.into(),
            goto_file.to_owned().into_os_string(), // input
            goto_file.to_owned().into_os_string(), // output
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
            "--no-malloc-may-fail".into(),
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

    /// Apply annotated function contracts and loop contracts with goto-instrument.
    pub fn instrument_contracts(
        &self,
        harness: &HarnessMetadata,
        is_loop_contracts_enabled: bool,
        file: &Path,
    ) -> Result<()> {
        // Do nothing if neither loop contracts nor function contracts is enabled.
        if !is_loop_contracts_enabled && harness.contract.is_none() {
            return Ok(());
        }

        let mut args: Vec<OsString> =
            vec!["--dfcc".into(), (&harness.mangled_name).into(), "--no-malloc-may-fail".into()];

        if is_loop_contracts_enabled {
            args.append(&mut vec![
                "--apply-loop-contracts".into(),
                "--loop-contracts-no-unwind".into(),
                // Because loop contracts now are wrapped in a closure which will be a side-effect expression in CBMC even they
                // may not contain side-effect. So we disable the side-effect check for now and will implement a better check
                // instead of simply rejecting function calls and statement expressions.
                // See issue: diffblue/cbmc#8393
                "--disable-loop-contracts-side-effect-check".into(),
            ]);
        }

        if let Some(assigns) = harness.contract.as_ref() {
            args.push("--enforce-contract".into());
            args.push(assigns.contracted_function_name.as_str().into());

            if let Some(tracker) = &assigns.recursion_tracker {
                args.push("--nondet-static-exclude".into());
                args.push(tracker.as_str().into());
            }
        }

        args.push(file.into());
        args.push(file.into());

        self.call_goto_instrument(&args)
    }

    /// Generate a .demangled.c file from the .c file using the `prettyName`s from the symbol table
    ///
    /// Currently, only top-level function names and (most) type names are demangled.
    /// For local variables, it would be more complicated than a simple search and replace to obtain the demangled name.
    pub fn demangle_c(
        &self,
        pretty_name_map_file: &impl AsRef<Path>,
        c_file: &Path,
        demangled_file: &Path,
    ) -> Result<()> {
        let mut c_code = std::fs::read_to_string(c_file)?;
        let reader = BufReader::new(File::open(pretty_name_map_file)?);
        let value: serde_json::Value = serde_json::from_reader(reader)?;
        let pretty_name_map = value.as_object().unwrap();
        for (name, pretty_name) in pretty_name_map {
            if let Some(pretty_name) = pretty_name.as_str() {
                // Struct names start with "tag-", but this prefix is not used in the GotoC files, so we strip it.
                // If there is no such prefix, we leave the name unchanged.
                let name = name.strip_prefix("tag-").unwrap_or(name);
                if !pretty_name.is_empty() && pretty_name != name {
                    c_code = c_code.replace(name, pretty_name);
                }
            }
        }
        std::fs::write(demangled_file, c_code)?;
        Ok(())
    }

    /// Non-public helper function to actually do the run of goto-instrument
    fn call_goto_instrument<S: AsRef<OsStr>>(
        &self,
        args: impl IntoIterator<Item = S>,
    ) -> Result<()> {
        // TODO get goto-instrument path from self
        let mut cmd = Command::new("goto-instrument");
        cmd.args(args);

        self.run_suppress(cmd)
    }
}
