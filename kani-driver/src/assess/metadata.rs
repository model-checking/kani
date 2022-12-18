// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::table_builder::TableBuilder;
use super::table_failure_reasons::FailureReasonsTableRow;
use super::table_promising_tests::PromisingTestsTableRow;
use super::table_unsupported_features::UnsupportedFeaturesTableRow;
use super::AssessArgs;

/// The structure of `.kani-assess-metadata.json` files, which are emitted for each crate.
/// This is not a stable interface.
#[derive(Deserialize)]
pub struct AssessMetadata {
    pub unsupported_features: Vec<UnsupportedFeaturesTableRow>,
    pub failure_reasons: Vec<FailureReasonsTableRow>,
    pub promising_tests: Vec<PromisingTestsTableRow>,
}

/// This should be kept identical to [`AssessMetadata`], but lifetimes will differ.
#[derive(Serialize)]
pub struct AssessMetadataOutput<'a> {
    pub unsupported_features: Vec<&'a UnsupportedFeaturesTableRow>,
    pub failure_reasons: Vec<&'a FailureReasonsTableRow>,
    pub promising_tests: Vec<&'a PromisingTestsTableRow>,
}

/// This should be kept identical to [`AssessMetadata`], but wrapper types will differ.
pub struct AssessMetadataAggregate {
    pub unsupported_features: TableBuilder<UnsupportedFeaturesTableRow>,
    pub failure_reasons: TableBuilder<FailureReasonsTableRow>,
    pub promising_tests: TableBuilder<PromisingTestsTableRow>,
}

pub(crate) fn write_metadata(args: &AssessArgs, build: AssessMetadataOutput) -> Result<()> {
    if let Some(path) = &args.emit_metadata {
        let out_file = File::create(&path)?;
        let writer = BufWriter::new(out_file);
        // use pretty for now to keep things readable and debuggable, but this should change eventually
        serde_json::to_writer_pretty(writer, &build)?;
    }
    Ok(())
}

pub(crate) fn write_partial_metadata(
    args: &AssessArgs,
    unsupported_features: TableBuilder<UnsupportedFeaturesTableRow>,
) -> Result<()> {
    write_metadata(
        args,
        AssessMetadataOutput {
            unsupported_features: unsupported_features.build(),
            failure_reasons: vec![],
            promising_tests: vec![],
        },
    )
}

pub(crate) fn read_metadata(path: &Path) -> Result<AssessMetadata> {
    // this function already exists, but a proxy here helps find it :)
    crate::metadata::from_json(path)
}

pub(crate) fn aggregate_metadata(metas: Vec<AssessMetadata>) -> AssessMetadataAggregate {
    let mut result = AssessMetadataAggregate {
        unsupported_features: TableBuilder::new(),
        failure_reasons: TableBuilder::new(),
        promising_tests: TableBuilder::new(),
    };
    for meta in metas {
        for item in meta.unsupported_features {
            result.unsupported_features.add(item);
        }
        for item in meta.failure_reasons {
            result.failure_reasons.add(item);
        }
        for item in meta.promising_tests {
            result.promising_tests.add(item);
        }
    }
    result
}
