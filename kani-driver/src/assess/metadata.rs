// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Assess metadata. This format is shared between 'assess' and 'assess scan'.
//! Assess produces this for one workspace, scan for several.
//! It is not a stable file format: it is meant for assess to directly communicate
//! from assess subprocesses to a parent scan process.
//! We can build other tools that make use of it, but they need to be built for or distributed
//! with the specific version of Kani.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::AssessArgs;
use super::table_builder::TableBuilder;
use super::table_failure_reasons::FailureReasonsTableRow;
use super::table_promising_tests::PromisingTestsTableRow;
use super::table_unsupported_features::UnsupportedFeaturesTableRow;

/// The structure of `.kani-assess-metadata.json` files. This is a the structure for both
/// assess (standard) and scan. It it meant to hold results for one or more packages.
///
/// This is not a stable interface.
#[derive(Serialize, Deserialize)]
pub struct AssessMetadata {
    /// Kani version that was used to generate the metadata.
    #[serde(rename = "kani_version")]
    pub version: String,
    /// Contains an error message in cases where it failed the execution.
    pub error: Option<SessionError>,
    /// Report on the presence of `codegen_unimplemented` in the analyzed packages
    pub unsupported_features: TableBuilder<UnsupportedFeaturesTableRow>,
    /// Report of the reasons why tests could not be analyzed by Kani
    pub failure_reasons: TableBuilder<FailureReasonsTableRow>,
    /// Report on the tests that Kani can successfully analyze
    pub promising_tests: TableBuilder<PromisingTestsTableRow>,
}

impl AssessMetadata {
    pub fn new(
        unsupported_features: TableBuilder<UnsupportedFeaturesTableRow>,
        failure_reasons: TableBuilder<FailureReasonsTableRow>,
        promising_tests: TableBuilder<PromisingTestsTableRow>,
    ) -> AssessMetadata {
        AssessMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            error: None,
            unsupported_features,
            failure_reasons,
            promising_tests,
        }
    }

    pub fn from_error(err: &dyn std::error::Error) -> AssessMetadata {
        let error = Some(SessionError {
            root_cause: err.source().map(|e| format!("{e:#}")),
            msg: err.to_string(),
        });
        AssessMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            error,
            unsupported_features: TableBuilder::new(),
            failure_reasons: TableBuilder::new(),
            promising_tests: TableBuilder::new(),
        }
    }
    pub fn empty() -> AssessMetadata {
        AssessMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            error: None,
            unsupported_features: TableBuilder::new(),
            failure_reasons: TableBuilder::new(),
            promising_tests: TableBuilder::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionError {
    pub root_cause: Option<String>,
    pub msg: String,
}

/// If given the argument to so do, write the assess metadata to the target file.
pub(crate) fn write_metadata(args: &AssessArgs, metadata: AssessMetadata) -> Result<()> {
    if let Some(path) = &args.emit_metadata {
        let out_file = File::create(path)?;
        let writer = BufWriter::new(out_file);
        // use pretty for now to keep things readable and debuggable, but this should change eventually
        serde_json::to_writer_pretty(writer, &metadata)?;
    }
    Ok(())
}

/// Read assess metadata from a file.
pub(crate) fn read_metadata(path: &Path) -> Result<AssessMetadata> {
    // this function already exists, but a proxy here helps find it :)
    crate::metadata::from_json(path)
}

/// Given assess metadata from several sources, aggregate them into a single strcture.
///
/// This is not a complicated operation, because the assess metadata structure is meant
/// to accomodate multiple packages already, so we're just "putting it together".
pub(crate) fn aggregate_metadata(metas: Vec<AssessMetadata>) -> AssessMetadata {
    let mut result = AssessMetadata::empty();
    for meta in metas {
        for item in meta.unsupported_features.build() {
            result.unsupported_features.add(item.clone());
        }
        for item in meta.failure_reasons.build() {
            result.failure_reasons.add(item.clone());
        }
        for item in meta.promising_tests.build() {
            result.promising_tests.add(item.clone());
        }
    }
    result
}
