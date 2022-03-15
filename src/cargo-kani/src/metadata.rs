// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use kani_metadata::{InternedString, KaniMetadata, TraitDefinedMethod, VtableCtxResults};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use crate::session::KaniSession;

/// From either a file or a path with multiple files, output the CBMC restrictions file we should use.
pub fn collect_and_link_function_pointer_restrictions(
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

/// Deserialize a *.restrictions.json file
fn read_restrictions(path: &Path) -> Result<VtableCtxResults> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let restrictions = serde_json::from_reader(reader)?;
    Ok(restrictions)
}

/// Deserialize a *.restrictions.json file
fn read_kani_metadata(path: &Path) -> Result<KaniMetadata> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let restrictions = serde_json::from_reader(reader)?;
    Ok(restrictions)
}

/// Consumes a vector of parsed metadata, and produces a combined structure
fn merge_kani_metadata(files: Vec<KaniMetadata>) -> KaniMetadata {
    let mut result = KaniMetadata { proof_harnesses: Vec::new() };
    for md in files {
        // Note that we're taking ownership of the original vec, and so we can move the data into the new data structure.
        result.proof_harnesses.extend(md.proof_harnesses);
    }
    result
}

impl KaniSession {
    /// Reads a collection of kani-metadata.json files and merges the results.
    pub fn collect_kani_metadata(&self, files: &[PathBuf]) -> Result<KaniMetadata> {
        if self.args.dry_run {
            // Mock an answer
            Ok(KaniMetadata { proof_harnesses: vec![] })
        } else {
            // TODO: one possible future improvement here would be to return some kind of Lazy
            // value, that only computes this metadata it turns out we need it.
            let results: Result<Vec<_>, _> = files.iter().map(|x| read_kani_metadata(x)).collect();
            Ok(merge_kani_metadata(results?))
        }
    }

    /// Determine which function to use as entry point, based on command-line arguments and kani-metadata.
    pub fn determine_target_function(&self, metadata: &KaniMetadata) -> Result<String> {
        if let Some(name) = &self.args.function {
            // --function is untranslated
            return Ok(name.to_string());
        }
        if let Some(name) = &self.args.harness {
            // Linear search, since this is only ever called once
            if let Some(harness) = metadata.proof_harnesses.iter().find(|x| x.pretty_name == *name)
            {
                return Ok(harness.mangled_name.to_string());
            }
            bail!("A proof harness named {} was not found", name);
        }
        Ok("main".to_string()) // TODO
    }
}
