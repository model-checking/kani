// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::session::KaniSession;
use crate::util::alter_extension;

use kani_metadata::{InternedString, TraitDefinedMethod, VtableCtxResults};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};

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

        if self.args.gen_c {
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

/// Collect all vtable restriction metadata together, and write one combined output in CBMC's format
fn link_function_pointer_restrictions(
    data_per_crate: Vec<VtableCtxResults>,
    output_filename: &Path,
) -> Result<()> {
    // Combine all method possibilities into one global mapping
    let mut combined_possible_methods: HashMap<TraitDefinedMethod, Vec<InternedString>> =
        HashMap::new();
    for crate_data in &data_per_crate {
        for entry in &crate_data.possible_methods {
            combined_possible_methods
                .insert(entry.trait_method.clone(), entry.possibilities.clone());
        }
    }

    // Emit a restriction for every call site
    let mut output = HashMap::new();
    for crate_data in data_per_crate {
        for call_site in crate_data.call_sites {
            // CBMC Now supports referencing callsites by label:
            // https://github.com/diffblue/cbmc/pull/6508
            let cbmc_call_site_name = format!("{}.{}", call_site.function_name, call_site.label);
            let trait_def = call_site.trait_method;

            // Look up all possibilities, defaulting to the empty set
            let possibilities =
                combined_possible_methods.get(&trait_def).unwrap_or(&vec![]).clone();
            output.insert(cbmc_call_site_name, possibilities);
        }
    }

    let f = File::create(output_filename)?;
    let f = BufWriter::new(f);
    serde_json::to_writer(f, &output)?;
    Ok(())
}

/// From either a file or a path with multiple files, output the CBMC restrictions file we should use.
fn collect_and_link_function_pointer_restrictions(
    path: &Path,
    output_filename: &Path,
) -> Result<()> {
    let md = std::fs::metadata(path)?;

    // Fill with data from all files in that path with the expected suffix
    let mut per_crate_restrictions = Vec::new();

    if md.is_dir() {
        for element in path.read_dir()? {
            let path = element?.path();
            if path.as_os_str().to_str().unwrap().ends_with(".restrictions.json") {
                let restrictions = read_restrictions(&path)?;
                per_crate_restrictions.push(restrictions);
            }
        }
    } else if md.is_file() {
        assert!(path.as_os_str().to_str().unwrap().ends_with(".restrictions.json"));
        let restrictions = read_restrictions(path)?;
        per_crate_restrictions.push(restrictions);
    } else {
        unreachable!("Path must be restrcitions file or directory containing restrictions files")
    }

    link_function_pointer_restrictions(per_crate_restrictions, output_filename)
}

/// Deserialize a *.restrictions.json file
fn read_restrictions(path: &Path) -> Result<VtableCtxResults> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let restrictions = serde_json::from_reader(reader)?;
    Ok(restrictions)
}
