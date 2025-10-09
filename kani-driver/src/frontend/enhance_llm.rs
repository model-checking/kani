// frontend/enhance_llm.rs
use anyhow::{Result, Context, bail};
use serde_json::{json, Value};
use crate::frontend::json_handler::JsonHandler;

// ============================================================================
// CONFIGURATION - Edit these values as needed
// ============================================================================

/// Hugging Face Inference API endpoint
const HF_API_URL: &str = "https://api-inference.huggingface.co/models";

/// Hugging Face API token
/// Get your free token at: https://huggingface.co/settings/tokens
/// TODO: Replace with token or set HF_TOKEN environment variable
const HF_API_TOKEN: &str = "YOUR_HF_TOKEN_HERE";

/// Model to use - Microsoft Phi-3 Mini (free, no rate limits for basic use)
/// Other free options:
/// - "microsoft/Phi-3-mini-4k-instruct" (3.8B params, very fast)
/// - "google/gemma-2-2b-it" (2B params, efficient)
/// - "meta-llama/Llama-3.2-3B-Instruct" (3B params, high quality)
/// - "mistralai/Mistral-7B-Instruct-v0.3" (7B params, powerful)
const HF_MODEL: &str = "microsoft/Phi-3-mini-4k-instruct";

/// System prompt for LLM - Defines the AI's role and expertise
const SYSTEM_PROMPT: &str = r#"You are an expert software verification analyst specializing in Kani, a formal verification tool for Rust programs. Your role is to analyze verification results and provide clear, actionable insights for developers.

Your expertise includes:
- Understanding Kani proof harnesses and verification checks
- Interpreting CBMC (Bounded Model Checker) outputs and statistics
- Identifying root causes of verification failures
- Explaining assertion failures, panics, and unsafe code issues
- Providing concrete recommendations for fixing verification failures

When analyzing results, you should:
1. Summarize the overall verification status clearly
2. Identify and explain each failure in detail
3. Provide the exact location of issues (file, line, column)
4. Suggest specific fixes or debugging approaches
5. Highlight any performance concerns (long runtimes, high memory usage)
6. Use clear, developer-friendly language avoiding unnecessary jargon

Always respond with valid JSON following this exact structure:
{
  "executive_summary": "Brief 2-3 sentence overview of verification results",
  "overall_status": "success" | "failure" | "partial",
  "key_findings": [
    "Finding 1: Brief statement",
    "Finding 2: Brief statement"
  ],
  "detailed_analysis": {
    "harnesses": [
      {
        "harness_name": "name of the harness",
        "status": "success" | "failure",
        "summary": "What this harness verifies",
        "issues": [
          {
            "severity": "critical" | "high" | "medium" | "low",
            "type": "assertion_failure" | "panic" | "overflow" | "other",
            "description": "Clear explanation of what went wrong",
            "location": "file:line:column",
            "root_cause": "Why this happened",
            "recommendation": "How to fix it"
          }
        ]
      }
    ]
  },
  "performance_insights": {
    "total_duration": "human readable time",
    "slowest_harness": "name and duration",
    "resource_usage": "any concerns about memory, solver time, etc."
  },
  "recommendations": [
    "Actionable recommendation 1",
    "Actionable recommendation 2"
  ],
  "next_steps": "What the developer should do next"
}"#;

/// User prompt template for LLM analysis
/// The placeholder {verification_results} will be replaced with actual data
const USER_PROMPT_TEMPLATE: &str = r#"Please analyze the following Kani verification results and provide a comprehensive, developer-friendly analysis.

The JSON data contains:
- metadata: Information about the Kani version, target platform, and build configuration
- project: Details about the Rust crate being verified
- harness_metadata: Metadata about each proof harness (location, attributes, contracts)
- verification_results: Summary and detailed results of each verification check
- CBMC: Low-level bounded model checker statistics and timing data
- Summary: High-level summary of verification outcomes

Focus on:
1. What verification checks passed or failed
2. Exact locations of any failures (file paths, line numbers)
3. Root causes of failures based on the error descriptions
4. Practical recommendations for fixing issues
5. Any performance concerns

Here are the verification results to analyze:

{verification_results}

Respond with valid JSON only, following the structure specified in your system instructions."#;

// ============================================================================
// MAIN FUNCTION
// ============================================================================

/// Enhance the JSON handler with LLM-powered analysis of verification results
///
/// This function:
/// 1. Extracts verification results from the handler
/// 2. Sends them to Hugging Face Inference API (free) for analysis
/// 3. Adds the LLM insights back to the handler
///
/// # Arguments
/// * `handler` - JSON handler containing verification results
///
/// # Returns
/// * `Ok(())` - Successfully added LLM insights
/// * `Err(_)` - Failed to analyze or add insights
pub fn enhance_llm(handler: &mut JsonHandler) -> Result<()> {
    eprintln!("=== Starting LLM Enhancement ===");
    eprintln!("Step 1: Extracting verification results...");
    let verification_results = extract_verification_results(handler)?;
    eprintln!("  ✓ Extracted {} bytes of verification data", serde_json::to_string(&verification_results)?.len());
    eprintln!("Step 2: Calling LLM for analysis...");
    let llm_analysis = call_llm_for_analysis(&verification_results)?;
    eprintln!("  ✓ LLM analysis completed successfully");
    eprintln!("Step 3: Adding LLM insights to handler...");
    handler.add_item("llm_insights", llm_analysis);
    eprintln!("=== LLM Enhancement Complete ===\n");
    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Extract verification results from the JSON handler
///
/// # Arguments
/// * `handler` - JSON handler to extract from
///
/// # Returns
/// * `Ok(Value)` - Verification results as JSON
/// * `Err(_)` - Failed to extract results
fn extract_verification_results(handler: &JsonHandler) -> Result<Value> {
    let results = handler.get_item("verification_results").context("No verification_results found in handler")?.clone();
    if results.is_null() {
        bail!("verification_results is null");
    }
    eprintln!("  Found verification results: {} items", if results.is_array() { results.as_array().map(|a| a.len()).unwrap_or(0) } else if results.is_object() { results.as_object().map(|o| o.len()).unwrap_or(0) } else { 0 });
    Ok(results)
}

/// Call LLM API to analyze verification results
///
/// Uses Hugging Face's free Inference API
/// Get your free token at: https://huggingface.co/settings/tokens
///
/// Note: This function requires an HTTP client library. 
/// Options:
/// 1. Add reqwest to Cargo.toml: reqwest = { version = "0.11", features = ["blocking", "json"] }
/// 2. Add ureq to Cargo.toml: ureq = { version = "2.9", features = ["json"] }
/// 3. Use curl command line (see make_curl_request function below)
///
/// # Arguments
/// * `verification_results` - Verification results to analyze
///
/// # Returns
/// * `Ok(Value)` - LLM analysis as structured JSON
/// * `Err(_)` - API call or parsing failed
fn call_llm_for_analysis(verification_results: &Value) -> Result<Value> {
    eprintln!("  Preparing LLM request...");
    
    // Get API token from environment variable or constant
    let api_key = std::env::var("HF_TOKEN")
        .or_else(|_| std::env::var("HUGGING_FACE_TOKEN"))
        .unwrap_or_else(|_| HF_API_TOKEN.to_string());
    
    if api_key == "YOUR_HF_TOKEN_HERE" || api_key.is_empty() {
        bail!("Please set HF_TOKEN environment variable or update HF_API_TOKEN constant.\nGet your free token at: https://huggingface.co/settings/tokens");
    }
    
    // Get API URL and model from environment variables or constants
    let model = std::env::var("HF_MODEL").unwrap_or_else(|_| HF_MODEL.to_string());
    let api_url = format!("{}/{}", HF_API_URL, model);
    
    eprintln!("  API URL: {}", api_url);
    eprintln!("  Model: {}", model);
    
    // Prepare the prompt
    let verification_json = serde_json::to_string_pretty(verification_results).context("Failed to serialize verification results")?;
    let user_prompt = USER_PROMPT_TEMPLATE.replace("{verification_results}", &verification_json);
    
    // Build the request body for Hugging Face API
    let request_body = json!({
        "inputs": format!("<|system|>\n{}<|end|>\n<|user|>\n{}<|end|>\n<|assistant|>", 
                         SYSTEM_PROMPT, user_prompt),
        "parameters": {
            "max_new_tokens": 2000,
            "temperature": 0.3,
            "top_p": 0.95,
            "do_sample": true,
            "return_full_text": false
        },
        "options": {
            "use_cache": false,
            "wait_for_model": true
        }
    });
    
    eprintln!("  Sending HTTP POST request...");
    
    // Make the HTTP request - using curl command as a fallback
    let response_json = make_curl_request(&api_url, &api_key, &request_body)?;
    
    // Extract generated text from response
    // Hugging Face API returns either:
    // - Array format: [{"generated_text": "..."}]
    // - Object format: {"generated_text": "..."}
    let content = if response_json.is_array() {
        response_json[0]["generated_text"]
            .as_str()
            .context("Invalid response format from LLM API")?
    } else {
        response_json["generated_text"]
            .as_str()
            .context("Invalid response format from LLM API")?
    };
    
    eprintln!("  ✓ Received response ({} chars)", content.len());
    
    // Parse the LLM response into structured JSON
    let analysis = parse_llm_response(content)?;
    Ok(analysis)
}

/// Make HTTP request using curl command
///
/// This is a fallback method that uses the curl command-line tool.
/// Works on most systems without requiring additional Rust dependencies.
///
/// # Arguments
/// * `url` - API endpoint URL
/// * `api_key` - Authentication token
/// * `body` - Request body as JSON
///
/// # Returns
/// * `Ok(Value)` - Response as JSON
/// * `Err(_)` - Request failed
fn make_curl_request(url: &str, api_key: &str, body: &Value) -> Result<Value> {
    use std::process::Command;
    use std::io::Write;
    
    // Create a temporary file for the request body
    let mut temp_file = tempfile::NamedTempFile::new()
        .context("Failed to create temporary file")?;
    
    let body_str = serde_json::to_string(body)
        .context("Failed to serialize request body")?;
    
    temp_file.write_all(body_str.as_bytes())
        .context("Failed to write request body to temp file")?;
    
    let temp_path = temp_file.path();
    
    // Execute curl command
    let output = Command::new("curl")
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("-H")
        .arg(format!("Authorization: Bearer {}", api_key))
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("--data-binary")
        .arg(format!("@{}", temp_path.display()))
        .arg("--max-time")
        .arg("120")
        .arg("--silent")
        .arg("--show-error")
        .output()
        .context("Failed to execute curl command. Make sure curl is installed.")?;
    
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        
        // Check for specific HTTP status codes in stderr
        if error_msg.contains("401") {
            bail!("LLM API error (401): Authentication failed. Please check your HF_TOKEN is valid.\nGet a new token at: https://huggingface.co/settings/tokens");
        } else if error_msg.contains("503") {
            eprintln!("  ⚠ Model is loading... this may take 20-60 seconds on first use.");
            eprintln!("  The model will be cached for future requests.");
            bail!("LLM API error (503): Model is currently loading. Please try again in a moment.");
        }
        
        bail!("curl command failed: {}", error_msg);
    }
    
    let response_str = String::from_utf8(output.stdout)
        .context("Failed to parse curl response as UTF-8")?;
    
    let response_json: Value = serde_json::from_str(&response_str)
        .context("Failed to parse LLM API response as JSON")?;
    
    eprintln!("  Response received successfully");
    
    Ok(response_json)
}

/// Parse LLM response text into structured JSON
///
/// # Arguments
/// * `response_text` - Raw text response from LLM
///
/// # Returns
/// * `Ok(Value)` - Parsed JSON
/// * `Err(_)` - Parsing failed
fn parse_llm_response(response_text: &str) -> Result<Value> {
    eprintln!("  Parsing LLM response...");
    let cleaned = response_text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let parsed: Value = serde_json::from_str(cleaned).context("Failed to parse LLM response as JSON")?;
    if !parsed.is_object() {
        bail!("LLM response is not a JSON object");
    }
    eprintln!("  ✓ Successfully parsed LLM response");
    Ok(parsed)
}