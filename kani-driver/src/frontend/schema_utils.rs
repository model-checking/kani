// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Utility functions for creating structured JSON schemas
//! This module contains helper functions to convert Kani internal structures to JSON

use crate::call_cbmc::VerificationStatus;
use crate::harness_runner::HarnessResult;
use crate::frontend::JsonHandler;
use kani_metadata::HarnessMetadata;
use serde_json::{Value, json};
use anyhow::Result;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use crate::project::Project;

/// Creates structured JSON metadata for an export run
/// This utility function captures basic environment for the whole session
pub fn create_metadata_json() -> Value {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    let kani_version = env!("CARGO_PKG_VERSION");
    let target = " ";
    let build_mode = if cfg!(debug_assertions) { "debug" } else { "release" };

    json!({
    "version": "1.0",
    "timestamp": timestamp,
    "kani_version": kani_version,
    "target": target,
    "build_mode": build_mode,
    })
}

/// Creates structured JSON metadata for the project
/// This utility function captures detailed info for the project
pub fn create_project_metadata_json(project: &Project) -> Value {
    json!({
    "crate_name": project.metadata.iter()
    .map(|m| m.crate_name.clone())
    .collect::<Vec<String>>(),
    "workspace_root": project.outdir.clone(),
    })
}
/// Creates structured JSON metadata for a harness
/// This utility function separates harness metadata creation from the main verification logic
pub fn create_harness_metadata_json(h: &HarnessMetadata) -> Value {
    json!({
        "pretty_name": h.pretty_name, // use this as identifier
        "mangled_name": h.mangled_name,
        "crate_name": h.crate_name,
        "source": {
            "file": h.original_file,
            "start_line": h.original_start_line,
            "end_line": h.original_end_line
        },
        "goto_file": h.goto_file.as_ref().map(|p| p.to_string_lossy().to_string()),
        "attributes": {
            "kind": format!("{:?}", h.attributes.kind),
            "should_panic": h.attributes.should_panic,
        },
        "Contract":{
            "contracted_function_name": h.contract.as_ref().map(|c| format!("{:?}", c)),
            "recursion_tracker": ""
        },
        "has_loop_contracts": h.has_loop_contracts,
        "is_automatically_generated": h.is_automatically_generated,

    })
}

/// Creates verification result JSON with harness reference
/// This reduces duplication between harness metadata and verification results
pub fn create_verification_result_json(result: &HarnessResult) -> Value {
    // Extract detailed verification results as "checks"
    let checks = match &result.result.results {
        Ok(properties) => {
            properties.iter().enumerate().map(|(i, prop)| {
                json!({
                    "id": i + 1,
                    "function": prop.property_id.fn_name.as_ref().unwrap_or(&"unknown".to_string()),
                    "status": format!("{:?}", prop.status),
                    "description": prop.description,
                    "location": {
                        "file": prop.source_location.file.as_ref().unwrap_or(&"unknown".to_string()),
                        "line": prop.source_location.line.as_ref().unwrap_or(&"unknown".to_string()),
                        "column": prop.source_location.column.as_ref().unwrap_or(&"unknown".to_string()),
                    },
                    "category": prop.property_id.class,
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
        "duration_ms": (result.result.runtime.as_millis() as u64),
        "checks": checks,
    })
}

/// Creates a verification summary with clean structure
pub fn create_verification_summary_json(
    results: &[HarnessResult],
    selected: usize,
    status_label: &str,
) -> Value {
    let successful = results.iter().filter(|r| r.result.status == VerificationStatus::Success).count();
    let failed = results.len() - successful;
    let total_duration_ms: u64 = results.iter().map(|r| r.result.runtime.as_millis() as u64).sum();
    
    let verification_results: Vec<_> = results.iter().map(|r| create_verification_result_json(r)).collect();

    json!({
        "summary": {
            "total_harnesses": selected,
            "executed": results.len(),
            "status": status_label,
            "successful": successful,
            "failed": failed,
            "duration_ms": total_duration_ms
        },
        "results": verification_results
    })
}

/// Process harness results and enrich JSON handler with additional metadata.
/// This function handles the complex harness processing logic, combining verification results
/// with harness metadata to create enriched JSON output.
pub fn process_harness_results(
    handler: &mut JsonHandler,
    harnesses: &[&HarnessMetadata],
    results: &[HarnessResult],
) -> Result<()> {
    // The main verification results are handled by the harness runner
    for h in harnesses {
        let harness_result = results.iter().find(|r| r.harness.pretty_name == h.pretty_name);
        
        // Add error details for this harness
        if let Some(result) = harness_result {
            handler.add_item("error_details", match result.result.status {
                VerificationStatus::Failure => {
                    json!({
                        "has_errors": true,
                        "error_type": match result.result.failed_properties {
                            crate::call_cbmc::FailedProperties::None => "unknown_failure",
                            crate::call_cbmc::FailedProperties::PanicsOnly => "assertion_failure",
                            crate::call_cbmc::FailedProperties::Other => "verification_failure",
                        },
                        "failed_properties_type": format!("{:?}", result.result.failed_properties),
                        "exit_status": match &result.result.results {
                            Err(crate::call_cbmc::ExitStatus::Timeout) => "timeout".to_string(),
                            Err(crate::call_cbmc::ExitStatus::OutOfMemory) => "out_of_memory".to_string(),
                            Err(crate::call_cbmc::ExitStatus::Other(code)) => format!("exit_code_{}", code),
                            Ok(_) => "properties_failed".to_string()
                        }
                    })
                },
                VerificationStatus::Success => json!({
                    "has_errors": false
                })
            });
            
            // Add property details for this harness
            handler.add_harness_detail("property_details", json!({
                "property_details": match &result.result.results {
                    Ok(properties) => {
                        let total_properties = properties.len();
                        let passed_properties = properties.iter().filter(|p| matches!(p.status, crate::cbmc_output_parser::CheckStatus::Success)).count();
                        let failed_properties = properties.iter().filter(|p| matches!(p.status, crate::cbmc_output_parser::CheckStatus::Failure)).count();
                        
                        json!({
                            "total_properties": total_properties,
                            "passed": passed_properties,
                            "failed": failed_properties,
                            "unreachable": total_properties - passed_properties - failed_properties
                        })
                    },
                    Err(_) => json!({
                        "total_properties": 0,
                        "error": "Could not extract property details due to verification failure"
                    })
                }
            }));
        }
    }
    
    Ok(())
}
