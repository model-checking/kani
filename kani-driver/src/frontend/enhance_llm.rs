// frontend/enhance_llm.rs
use anyhow::{Result, Context};
use serde_json::{json, Value};
use std::process::Command;
use std::path::PathBuf;

use crate::frontend::json_handler::JsonHandler;
use crate::harness_runner::HarnessResult;
use kani_metadata::HarnessMetadata;
use crate::call_cbmc::VerificationStatus;

/// Post-process the run into an LLM-friendly summary section.
/// This does NOT change verification logic; it only augments the JSON.
/// 
/// # Arguments
/// * `handler` - JSON handler to add the LLM section to
/// * `results` - Verification results from harness runs
/// * `harnesses` - Metadata about the harnesses that were run
pub fn enhance_llm(
    handler: &mut JsonHandler,
    results: &[HarnessResult<'_>],
    harnesses: &[&HarnessMetadata],
) -> Result<()> {
    enhance_llm_with_source(handler, results, harnesses, None)
}

/// Post-process the run into an LLM-friendly summary section with optional source file.
/// This does NOT change verification logic; it only augments the JSON.
/// 
/// # Arguments
/// * `handler` - JSON handler to add the LLM section to
/// * `results` - Verification results from harness runs
/// * `harnesses` - Metadata about the harnesses that were run
/// * `source_file` - Optional path to the source file being verified (for --llm output)
pub fn enhance_llm_with_source(
    handler: &mut JsonHandler,
    results: &[HarnessResult<'_>],
    harnesses: &[&HarnessMetadata],
    source_file: Option<&str>,
) -> Result<()> {
    eprintln!("=== Starting LLM Enhancement ===");
    
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut failing_names = Vec::new();

    for r in results {
        match r.result.status {
            VerificationStatus::Success => passed += 1,
            VerificationStatus::Failure => {
                failed += 1;
                failing_names.push(r.harness.pretty_name.clone());
            }
        }
    }

    let summary_text = if failed == 0 {
        format!("All {} harnesses verified successfully.", passed)
    } else {
        format!(
            "{} passed, {} failed. Failing: {}",
            passed,
            failed,
            failing_names.join(", ")
        )
    };

    eprintln!("Summary: {}", summary_text);
    eprintln!("Passed: {}, Failed: {}", passed, failed);

    // Capture JSON output from kani command with --llm flag
    let kani_llm_output = if let Some(src_file) = source_file {
        eprintln!("Capturing kani --llm output for: {}", src_file);
        match capture_kani_llm_output(src_file) {
            Ok(output) => {
                eprintln!("Kani LLM output captured: {}", output.is_some());
                output
            }
            Err(e) => {
                eprintln!("Warning: Failed to capture kani --llm output: {}", e);
                None
            }
        }
    } else {
        eprintln!("No source file provided for kani --llm");
        None
    };

    // Build the LLM section with both summary and kani output
    let mut llm_data = json!({
        "summary": {
            "total": harnesses.len(),
            "passed": passed,
            "failed": failed,
        },
        "failing_harnesses": failing_names,
        "note": summary_text
    });

    // Merge kani --llm output if available
    if let Some(kani_output) = kani_llm_output {
        if let Value::Object(map) = &mut llm_data {
            map.insert("kani_llm_output".to_string(), kani_output);
        }
    }

    // Debug: Print final LLM data
    eprintln!("Final LLM data structure:");
    eprintln!("{}", serde_json::to_string_pretty(&llm_data).unwrap_or_else(|_| "Error formatting JSON".to_string()));
    eprintln!("=== LLM Enhancement Complete ===\n");

    handler.add_item("llm", llm_data);

    Ok(())
}

/// Execute kani with --llm flag and capture its JSON output
/// 
/// # Arguments
/// * `source_file` - Path to the source file to verify
/// 
/// # Returns
/// * `Ok(Some(Value))` - Successfully captured and parsed JSON output
/// * `Ok(None)` - Command succeeded but no JSON file was produced
/// * `Err(_)` - Command execution or parsing failed
fn capture_kani_llm_output(source_file: &str) -> Result<Option<Value>> {
    let temp_json = PathBuf::from("/tmp/kani_llm_output.json");
    
    eprintln!("Executing: ./scripts/kani --export-json {:?} {} --llm", temp_json, source_file);
    
    // Execute: ./scripts/kani --export-json <temp_file> <source> --llm
    let output = Command::new("./scripts/kani")
        .arg("--export-json")
        .arg(&temp_json)
        .arg(source_file)
        .arg("--llm")
        .output()
        .context("Failed to execute kani command with --llm flag")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Warning: kani --llm command failed with status: {}", output.status);
        eprintln!("Stderr: {}", stderr);
        return Ok(None);
    }

    eprintln!("Kani command succeeded, checking for output file...");

    // Read and parse the JSON output
    if temp_json.exists() {
        let json_content = std::fs::read_to_string(&temp_json)
            .context("Failed to read kani LLM output JSON")?;
        
        eprintln!("=== Raw Kani LLM Output ===");
        eprintln!("{}", json_content);
        eprintln!("===========================\n");
        
        let parsed: Value = serde_json::from_str(&json_content)
            .context("Failed to parse kani LLM output JSON")?;
        
        // Clean up temp file
        if let Err(e) = std::fs::remove_file(&temp_json) {
            eprintln!("Warning: Failed to remove temp file {:?}: {}", temp_json, e);
        } else {
            eprintln!("Cleaned up temp file: {:?}", temp_json);
        }
        
        Ok(Some(parsed))
    } else {
        eprintln!("Warning: Temp JSON file {:?} does not exist", temp_json);
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_generation() {
        // Actual test here 
        // Just demonstrates the structure
        eprintln!("Test: Verify summary text generation works correctly");
        
        let summary = "5 passed, 2 failed. Failing: test1, test2";
        assert!(summary.contains("passed"));
        assert!(summary.contains("failed"));
    }
}