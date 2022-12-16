// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use std::path::Path;

use kani_metadata::{
    HarnessMetadata, InternedString, KaniMetadata, TraitDefinedMethod, VtableCtxResults,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use crate::session::KaniSession;
use serde::Deserialize;

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
                let restrictions = from_json(&path)?;
                per_crate_restrictions.push(restrictions);
            }
        }
    } else if md.is_file() {
        assert!(path.as_os_str().to_str().unwrap().ends_with(".restrictions.json"));
        let restrictions = from_json(path)?;
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

/// Deserialize a json file into a given structure
pub fn from_json<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let obj = serde_json::from_reader(reader)?;
    Ok(obj)
}

/// Consumes a vector of parsed metadata, and produces a combined structure
pub fn merge_kani_metadata(files: Vec<KaniMetadata>) -> KaniMetadata {
    let mut result = KaniMetadata {
        crate_name: "cbmc-linked".to_string(),
        proof_harnesses: vec![],
        unsupported_features: vec![],
        test_harnesses: vec![],
    };
    for md in files {
        // Note that we're taking ownership of the original vec, and so we can move the data into the new data structure.
        result.proof_harnesses.extend(md.proof_harnesses);
        // TODO: these should be merged via a map to aggregate them all
        // https://github.com/model-checking/kani/issues/1758
        result.unsupported_features.extend(md.unsupported_features);
        result.test_harnesses.extend(md.test_harnesses);
    }
    result
}

impl KaniSession {
    /// Determine which function to use as entry point, based on command-line arguments and kani-metadata.
    pub fn determine_targets(
        &self,
        all_harnesses: &[&HarnessMetadata],
    ) -> Result<Vec<HarnessMetadata>> {
        if let Some(name) = self.args.harness.clone().or(self.args.function.clone()) {
            // Linear search, since this is only ever called once
            let harness = find_proof_harness(&name, all_harnesses)?;
            return Ok(vec![harness.clone()]);
        }
        Ok(all_harnesses.iter().map(|md| (*md).clone()).collect())
    }
}

/// Sort harnesses such that for two harnesses in the same file, it is guaranteed that later
/// appearing harnesses get processed earlier.
/// This is necessary for the concrete playback feature (with in-place unit test modification)
/// because it guarantees that injected unit tests will not change the location of to-be-processed harnesses.
pub fn sort_harnesses_by_loc(harnesses: &[HarnessMetadata]) -> Vec<&HarnessMetadata> {
    let mut harnesses_clone: Vec<_> = harnesses.iter().by_ref().collect();
    harnesses_clone.sort_unstable_by(|harness1, harness2| {
        harness1
            .original_file
            .cmp(&harness2.original_file)
            .then(harness1.original_start_line.cmp(&harness2.original_start_line).reverse())
    });
    harnesses_clone
}

pub fn mock_proof_harness(
    name: &str,
    unwind_value: Option<u32>,
    krate: Option<&str>,
) -> HarnessMetadata {
    HarnessMetadata {
        pretty_name: name.into(),
        mangled_name: name.into(),
        crate_name: krate.unwrap_or("<unknown>").into(),
        original_file: "<unknown>".into(),
        original_start_line: 0,
        original_end_line: 0,
        unwind_value,
        goto_file: None,
    }
}

/// Search for a proof harness with a particular name.
/// At the present time, we use `no_mangle` so collisions shouldn't happen,
/// but this function is written to be robust against that changing in the future.
fn find_proof_harness<'a>(
    name: &str,
    harnesses: &'a [&HarnessMetadata],
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
            mock_proof_harness("check_one", None, None),
            mock_proof_harness("module::check_two", None, None),
            mock_proof_harness("module::not_check_three", None, None),
        ];
        let ref_harnesses = harnesses.iter().collect::<Vec<_>>();
        assert!(find_proof_harness("check_three", &ref_harnesses).is_err());
        assert!(
            find_proof_harness("check_two", &ref_harnesses).unwrap().mangled_name
                == "module::check_two"
        );
        assert!(
            find_proof_harness("check_one", &ref_harnesses).unwrap().mangled_name == "check_one"
        );
    }
}
