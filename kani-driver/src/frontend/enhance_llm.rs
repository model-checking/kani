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

/// System prompt for LLM
/// TODO: Add system prompt here
const SYSTEM_PROMPT: &str = r#""#;

/// User prompt template for LLM analysis
/// The placeholder {verification_results} will be replaced with actual data
/// TODO: Add your user prompt template here
const USER_PROMPT_TEMPLATE: &str = r#"{verification_results}"#;

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
    
    // Create HTTP client
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Failed to create HTTP client")?;
    
    // Send request to Hugging Face API
    let response = client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .context("Failed to send request to LLM API")?;
    
    let status = response.status();
    eprintln!("  Response status: {}", status);
    
    if !status.is_success() {
        let error_text = response.text().unwrap_or_else(|_| "Unable to read error response".to_string());
        
        // Provide helpful error messages
        if status.as_u16() == 401 {
            bail!("LLM API error (401): Authentication failed. Please check your HF_TOKEN is valid.\nGet a new token at: https://huggingface.co/settings/tokens");
        } else if status.as_u16() == 503 {
            eprintln!("  ⚠ Model is loading... this may take 20-60 seconds on first use.");
            eprintln!("  The model will be cached for future requests.");
            bail!("LLM API error (503): Model is currently loading. Please try again in a moment.");
        }
        
        bail!("LLM API error ({}): {}", status, error_text);
    }
    
    // Parse the API response
    let response_json: Value = response.json().context("Failed to parse LLM API response")?;
    
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