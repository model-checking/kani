// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use kani_metadata::{
    HarnessMetadata, InternedString, KaniMetadata, TraitDefinedMethod, VtableCtxResults,
};
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
            // value, that only computes this metadata if it turns out we need it.
            let results: Result<Vec<_>, _> =
                files.iter().map(|x| self.read_kani_metadata(x)).collect();
            Ok(merge_kani_metadata(results?))
        }
    }

    /// Determine which function to use as entry point, based on command-line arguments and kani-metadata.
    pub fn determine_targets(&self, metadata: &KaniMetadata) -> Result<Vec<HarnessMetadata>> {
        if let Some(name) = &self.args.function {
            // --function is untranslated, create a mock harness
            return Ok(vec![mock_proof_harness(name)]);
        }
        if let Some(name) = &self.args.harness {
            // Linear search, since this is only ever called once
            let harness = find_proof_harness(name, &metadata.proof_harnesses)?;
            return Ok(vec![harness.clone()]);
        }
        if metadata.proof_harnesses.is_empty() {
            // TODO: This could use a better error message, possibly with links to Kani documentation.
            // New users may encounter this and could use a pointer to how to write proof harnesses.
            bail!("No proof harnesses (functions with #[kani::proof]) were found to verify.");
        } else {
            Ok(metadata.proof_harnesses.clone())
        }
    }

    /// Deserialize a *.restrictions.json file
    fn read_kani_metadata(&self, path: &Path) -> Result<KaniMetadata> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let restrictions = serde_json::from_reader(reader)?;
        Ok(restrictions)
    }
}

fn mock_proof_harness(name: &str) -> HarnessMetadata {
    HarnessMetadata {
        pretty_name: name.into(),
        mangled_name: name.into(),
        original_file: "<unknown>".into(),
        original_line: "<unknown>".into(),
        unwind_value: None,
    }
}

/// Search for a proof harness with a particular name.
/// At the present time, we use `no_mangle` so collisions shouldn't happen,
/// but this function is written to be robust against that changing in the future.
fn find_proof_harness<'a>(
    name: &str,
    harnesses: &'a [HarnessMetadata],
) -> Result<&'a HarnessMetadata> {
    let mut result: Option<&'a HarnessMetadata> = None;
    for h in harnesses.iter() {
        // Either an exact match, or...
        let matches = h.pretty_name == *name || {
            // pretty_name will be things like `module::submodule::name_of_function`
            // and we want people to be able to specify `--harness name_of_function`
            if let Some(prefix) = h.pretty_name.strip_suffix(name) {
                prefix.ends_with("::")
            } else {
                false
            }
        };
        if matches {
            if let Some(other) = result {
                bail!(
                    "Conflicting proof harnesses named {}:\n {}\n {}",
                    name,
                    other.pretty_name,
                    h.pretty_name
                );
            } else {
                result = Some(h);
            }
        }
    }
    if let Some(x) = result {
        Ok(x)
    } else {
        bail!("A proof harness named {} was not found", name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_find_proof_harness() {
        let harnesses = vec![
            mock_proof_harness("check_one"),
            mock_proof_harness("module::check_two"),
            mock_proof_harness("module::not_check_three"),
        ];
        assert!(find_proof_harness("check_three", &harnesses).is_err());
        assert!(
            find_proof_harness("check_two", &harnesses).unwrap().mangled_name
                == "module::check_two"
        );
        assert!(find_proof_harness("check_one", &harnesses).unwrap().mangled_name == "check_one");
    }
}
