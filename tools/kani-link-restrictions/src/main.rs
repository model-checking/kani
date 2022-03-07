// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use cbmc::InternedString;
use kani_metadata::{TraitDefinedMethod, VtableCtxResults};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;

fn link_function_pointer_restrictions(data_per_crate: Vec<VtableCtxResults>) -> serde_json::Value {
    let mut output = HashMap::new();
    let mut combined_possible_methods: HashMap<TraitDefinedMethod, Vec<InternedString>> =
        HashMap::new();

    // Combine method possibilities
    for crate_data in &data_per_crate {
        for entry in &crate_data.possible_methods {
            combined_possible_methods
                .insert(entry.trait_method.clone(), entry.possibilities.clone());
        }
    }
    // Iterate call sites
    for crate_data in data_per_crate {
        for call_site in crate_data.call_sites {
            // CBMC Now supports referencing callsites by label:
            // https://github.com/diffblue/cbmc/pull/6508
            let cbmc_call_site_name = format!("{}.{}", call_site.function_name, call_site.label);
            let trait_def = call_site.trait_method;

            // Look up all possibilities, defaulting to the empty set
            if let Some(possibilities) = combined_possible_methods.get(&trait_def) {
                output.insert(cbmc_call_site_name, possibilities.clone());
            } else {
                output.insert(cbmc_call_site_name, Vec::<InternedString>::new());
            }
        }
    }
    serde_json::to_value(&output).unwrap()
}

pub fn main() {
    // We expected a single argument:
    // A path representing either the single file or a directory with multiple files
    let args: Vec<String> = env::args().collect();
    assert!(args.len() == 3);
    let path = &args[1];
    let outpath = &args[2];
    let md = std::fs::metadata(path).unwrap();

    // Fill with data from all files in that path with the expected suffix
    let mut per_crate_restrictions = Vec::new();

    if md.is_dir() {
        for element in std::path::Path::new(path).read_dir().unwrap() {
            let path = element.unwrap().path();
            if path.as_os_str().to_str().unwrap().ends_with("restrictions.json") {
                let file = File::open(path).unwrap();
                let reader = BufReader::new(file);
                let restrictions = serde_json::from_reader(reader).unwrap();
                per_crate_restrictions.push(restrictions);
            }
        }
    } else if md.is_file() {
        assert!(path.ends_with(".restrictions.json"));
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let restrictions = serde_json::from_reader(reader).unwrap();
        per_crate_restrictions.push(restrictions);
    } else {
        unreachable!("Path must be restrcitions file or directory containing restrictions files")
    }

    // Combine restrictions
    let f = File::create(outpath).unwrap();
    let linked = link_function_pointer_restrictions(per_crate_restrictions);
    serde_json::to_writer(f, &linked).unwrap();
}
