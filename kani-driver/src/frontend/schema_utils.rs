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
    let target = env!("TARGET");
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
            "contracted_function_name": h.contract.as_ref()
            .map(|c| c.contracted_function_name.as_str()),
            "recursion_tracker": h.contract.as_ref()
            .and_then(|c| c.recursion_tracker.as_ref()),
        },
        "has_loop_contracts": h.has_loop_contracts,
        "is_automatically_generated": h.is_automatically_generated,

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

/// Process harness results and enrich JSON handler with additional metadata.
/// This function handles the complex harness processing logic, combining verification results
/// with harness metadata to create enriched JSON output.
pub fn process_harness_results(
    handler: &mut JsonHandler,
    harnesses: &[&HarnessMetadata],
    results: &[HarnessResult],
) -> Result<()> {
    for h in harnesses {
        let harness_result = results.iter().find(|r| r.harness.pretty_name == h.pretty_name);
        let arr = handler.data["verification_runner_results"]["individual_harnesses"]
            .as_array_mut()
            .expect("individual_harnesses must be an array");

        // locate matching entry by harness_id and overwrite it
        let entry = arr.iter_mut().find(|v| {
            v.get("harness_id").and_then(|s| s.as_str()) == Some(h.pretty_name.as_str())
        }).expect("matching individual_harness not found");

        // Get the original verification details from the entry before overwriting
        let verification_details = entry.get("verification_details").cloned().unwrap_or(json!([]));
        let status = entry.get("status").and_then(|s| s.as_str()).unwrap_or("Unknown");

        *entry = json!({
            "harness_id": h.pretty_name,                    // Keep harness_id for consistency
            "name": h.pretty_name,                          // Also keep name for backward compatibility
            "status": status,                               // Preserve the verification status
            "verification_details": verification_details,   // Preserve verification details

            //original source location
            "original": {
                "file": h.original_file,
                "start_line": h.original_start_line,
                "end_line": h.original_end_line
            },

            // attributes
            "kind": format!("{:?}", h.attributes.kind),
            "should_panic": h.attributes.should_panic,
            "has_loop_contracts": h.has_loop_contracts,
            "is_automatically_generated": h.is_automatically_generated,
            "solver": h.attributes.solver.as_ref().map(|s| format!("{:?}", s)),
            "unwind_value": h.attributes.unwind_value,
            "contract": h.contract.as_ref().map(|c| format!("{:?}", c)),
            "stubs": h.attributes.stubs.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>(),
            "verified_stubs": h.attributes.verified_stubs,

            "summary": harness_result.map_or(json!(null), |result| json!({
                "total": 1,
                "status": match result.result.status {
                    VerificationStatus::Success => "completed",
                    VerificationStatus::Failure => "failed",
                }
            })),
            "timing": harness_result.map_or(json!(null), |result| json!({
                "cbmc_runtime": format!("{:.3}s", result.result.runtime.as_secs_f64())
            }))
        });
        
        // Add error details for this harness
        handler.add_item("error_details", harness_result.map_or(json!(null), |result| {
            match result.result.status {
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
            }
        }));
        
        // Add property details for this harness
        handler.add_harness_detail("property_details", json!({
            "property_details": harness_result.map_or(json!(null), |result| {
                match &result.result.results {
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
            })
        }));
    }
    
    Ok(())
}
