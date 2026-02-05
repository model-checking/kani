// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Tests for the schema_utils module
/// This module contains tests for the schema_utils module
/// and the json_handler module
use crate::call_cbmc::{ExitStatus, FailedProperties, VerificationResult, VerificationStatus};
use crate::cbmc_output_parser::{CheckStatus, Property, PropertyId, SourceLocation};
use crate::frontend::JsonHandler;
use crate::frontend::schema_utils::{
    add_runner_results_to_json, create_harness_metadata_json, create_metadata_json,
    create_project_metadata_json, create_verification_result_json,
    create_verification_summary_json,
};
use crate::harness_runner::HarnessResult;
use crate::project::Project;
use kani_metadata::{HarnessAttributes, HarnessKind, HarnessMetadata, KaniMetadata};
use std::path::PathBuf;
use std::time::Duration;
#[test]
fn test_create_metadata_json() {
    let json = create_metadata_json();

    assert!(json.is_object());
    assert_eq!(json["version"], "1.0");
    assert!(json["timestamp"].as_str().unwrap().contains('T'));
    assert!(["debug", "release"].contains(&json["build_mode"].as_str().unwrap()));
}

#[test]
fn test_create_project_metadata_json() {
    let mut project = Project::default();
    let metadata = KaniMetadata {
        crate_name: "sample_crate".to_string(),
        proof_harnesses: vec![],
        test_harnesses: vec![],
        unsupported_features: vec![],
        contracted_functions: vec![],
        autoharness_md: None,
    };
    project.outdir = PathBuf::from("/tmp/outdir");
    project.metadata.push(metadata);

    let json = create_project_metadata_json(&project);
    assert_eq!(json["crate_name"][0], "sample_crate");
    assert_eq!(json["workspace_root"], "/tmp/outdir");
}

#[test]
fn test_create_harness_metadata_json() {
    let harness = HarnessMetadata {
        pretty_name: "crate::mod::my_harness".to_string(),
        mangled_name: "mangled::harness".to_string(),
        crate_name: "sample_crate".to_string(),
        original_file: "src/lib.rs".to_string(),
        original_start_line: 10,
        original_end_line: 20,
        goto_file: Some(PathBuf::from("target/goto_file.goto")),
        attributes: HarnessAttributes::new(HarnessKind::Proof),
        contract: None,
        has_loop_contracts: true,
        is_automatically_generated: false,
    };

    let json = create_harness_metadata_json(&harness);

    assert_eq!(json["pretty_name"], "crate::mod::my_harness");
    assert_eq!(json["crate_name"], "sample_crate");
    assert_eq!(json["source"]["file"], "src/lib.rs");
    assert_eq!(json["source"]["start_line"], 10);
    assert_eq!(json["source"]["end_line"], 20);
    assert_eq!(json["has_loop_contracts"], true);
    assert_eq!(json["is_automatically_generated"], false);
}

#[test]
fn test_create_verification_result_json() {
    let harness = HarnessMetadata {
        pretty_name: "crate::my_harness".to_string(),
        mangled_name: "mangled_name".to_string(),
        crate_name: "sample_crate".to_string(),
        original_file: "src/lib.rs".to_string(),
        original_start_line: 1,
        original_end_line: 2,
        goto_file: None,
        attributes: HarnessAttributes::new(HarnessKind::Proof),
        contract: None,
        has_loop_contracts: false,
        is_automatically_generated: false,
    };

    let properties = vec![
        Property {
            property_id: PropertyId {
                id: 1,
                fn_name: Some("foo".to_string()),
                class: "safety".to_string(),
            },
            status: CheckStatus::Success,
            description: "no overflow".to_string(),
            source_location: SourceLocation {
                file: Some("src/lib.rs".to_string()),
                function: Some("foo".to_string()),
                line: Some("42".to_string()),
                column: Some("5".to_string()),
            },
            reach: None,
            trace: None,
        },
        Property {
            property_id: PropertyId {
                id: 2,
                fn_name: Some("bar".to_string()),
                class: "assertion".to_string(),
            },
            status: CheckStatus::Failure,
            description: "assert failed".to_string(),
            source_location: SourceLocation {
                file: Some("src/main.rs".to_string()),
                function: Some("bar".to_string()),
                line: Some("10".to_string()),
                column: Some("3".to_string()),
            },
            reach: None,
            trace: None,
        },
    ];

    let verification_result = VerificationResult {
        status: VerificationStatus::Failure,
        failed_properties: FailedProperties::Other,
        results: Ok(properties),
        runtime: Duration::from_millis(120),
        generated_concrete_test: false,
        coverage_results: None,
        cbmc_stats: None,
    };

    let harness_result = HarnessResult { harness: &harness, result: verification_result };

    let json = create_verification_result_json(&harness_result);

    // --- Assertions ---
    assert_eq!(json["harness_id"], "crate::my_harness");
    assert_eq!(json["status"], "Failure");
    assert_eq!(json["checks"][0]["function"], "foo");
    assert_eq!(json["checks"][1]["function"], "bar");
    assert_eq!(json["checks"][1]["location"]["file"], "src/main.rs");

    // Optional extra check
    assert!(json["duration_ms"].as_u64().unwrap() >= 100);
}

#[test]
fn test_create_verification_summary_json_real() {
    // Create a real harness
    let harness = HarnessMetadata {
        pretty_name: "foo::harness_ok".into(),
        mangled_name: "foo_harness_ok".into(),
        crate_name: "sample".into(),
        original_file: "src/lib.rs".into(),
        original_start_line: 10,
        original_end_line: 20,
        goto_file: None,
        attributes: HarnessAttributes::new(HarnessKind::Proof),
        contract: None,
        has_loop_contracts: false,
        is_automatically_generated: false,
    };

    // Create a VerificationResult
    let verification_result = VerificationResult::mock_success();

    let harness_result = HarnessResult { harness: &harness, result: verification_result };

    let json = create_verification_summary_json(&[harness_result], 1, "Completed");

    assert_eq!(json["summary"]["status"], "Completed");
    assert_eq!(json["summary"]["total_harnesses"], 1);
    assert_eq!(json["summary"]["executed"], 1);
    assert_eq!(json["summary"]["successful"], 1);
    assert_eq!(json["summary"]["failed"], 0);
    assert!(json["results"].is_array());
}

#[test]
fn test_add_runner_results_to_json_real() {
    let harness = HarnessMetadata {
        pretty_name: "bar::harness_fail".into(),
        mangled_name: "bar_harness_fail".into(),
        crate_name: "sample".into(),
        original_file: "src/main.rs".into(),
        original_start_line: 5,
        original_end_line: 15,
        goto_file: None,
        attributes: HarnessAttributes::new(HarnessKind::Proof),
        contract: None,
        has_loop_contracts: false,
        is_automatically_generated: false,
    };

    let verification_result = VerificationResult {
        status: VerificationStatus::Failure,
        failed_properties: FailedProperties::Other,
        results: Err(ExitStatus::Other(42)),
        runtime: Duration::from_millis(120),
        generated_concrete_test: false,
        coverage_results: None,
        cbmc_stats: None,
    };

    let harness_result = HarnessResult { harness: &harness, result: verification_result };

    let mut handler = JsonHandler::new(None);
    add_runner_results_to_json(&mut handler, &[harness_result], 1, "Failed");

    let summary = &handler.data["verification_results"]["summary"];
    assert_eq!(summary["status"], "Failed");
    assert_eq!(summary["failed"], 1);
}
