// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Utility functions for creating structured JSON schemas
//! This module contains helper functions to convert Kani internal structures to JSON

use crate::call_cbmc::VerificationStatus;
use crate::harness_runner::HarnessResult;
use kani_metadata::HarnessMetadata;
use serde_json::{Value, json};

/// Creates structured JSON metadata for a harness
/// This utility function separates harness metadata creation from the main verification logic
pub fn create_harness_metadata_json(h: &HarnessMetadata) -> Value {
    json!({
        "id": h.pretty_name,  // Use pretty_name as unique identifier
        "name": h.pretty_name,
        "mangled_name": h.mangled_name,
        "crate_name": h.crate_name,
        "source": {
            "file": h.original_file,
            "start_line": h.original_start_line,
            "end_line": h.original_end_line
        },
        "configuration": {
            "kind": format!("{:?}", h.attributes.kind),
            "should_panic": h.attributes.should_panic,
            "has_loop_contracts": h.has_loop_contracts,
            "is_automatically_generated": h.is_automatically_generated,
            "solver": h.attributes.solver.as_ref().map(|s| format!("{:?}", s)),
            "unwind_value": h.attributes.unwind_value,
            "contract": h.contract.as_ref().map(|c| format!("{:?}", c)),
            "stubs": h.attributes.stubs.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>(),
            "verified_stubs": h.attributes.verified_stubs
        },
        "artifacts": {
            "goto_file": h.goto_file.as_ref().map(|p| p.to_string_lossy().to_string())
        }
    })
}

/// Creates verification result JSON with harness reference
/// This reduces duplication between harness metadata and verification results
pub fn create_verification_result_json(result: &HarnessResult) -> Value {
    // Extract detailed verification results
    let verification_details = match &result.result.results {
        Ok(properties) => {
            properties.iter().enumerate().map(|(i, prop)| {
                json!({
                    "check_number": i + 1,
                    "function_name": prop.property_id.fn_name.as_ref().unwrap_or(&"unknown".to_string()),
                    "status": format!("{:?}", prop.status),
                    "description": prop.description,
                    "location": {
                        "file": prop.source_location.file.as_ref().unwrap_or(&"unknown".to_string()),
                        "line": prop.source_location.line.as_ref().unwrap_or(&"unknown".to_string()),
                        "column": prop.source_location.column.as_ref().unwrap_or(&"unknown".to_string()),
                    },
                    "class": prop.property_id.class,
                })
            }).collect::<Vec<_>>()
        },
        Err(_) => vec![],
    };

    json!({
        "harness_id": result.harness.pretty_name,  // Reference to harness instead of duplicating name
        "status": match result.result.status {
            VerificationStatus::Success => "Success",
            VerificationStatus::Failure => "Failure",
        },
        "verification_details": verification_details,
    })
}

/// Creates a verification summary with clean structure
pub fn create_verification_summary_json(
    results: &[HarnessResult],
    selected: usize,
    status_label: &str,
) -> Value {
    let details: Vec<_> = results.iter().map(|r| create_verification_result_json(r)).collect();

    json!({
        "selected": selected,
        "executed": results.len(),
        "status": status_label,
        "individual_harnesses": details,
    })
}
