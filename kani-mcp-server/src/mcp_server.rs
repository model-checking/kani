use crate::kani_wrapper::{KaniOptions, KaniWrapper, VerificationResult};
use crate::tools::{ToolResult, get_kani_tools};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Main MCP server for Kani integration
pub struct KaniMcpServer {
    kani: Arc<KaniWrapper>,
    last_result: Arc<Mutex<Option<VerificationResult>>>,
}

impl KaniMcpServer {
    pub fn new() -> Result<Self> {
        let kani = Arc::new(KaniWrapper::new()?);
        Ok(Self { kani, last_result: Arc::new(Mutex::new(None)) })
    }

    pub async fn run(self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader, stdin};

        let stdin = stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(request) => {
                    let response = self.handle_mcp_request(request).await;

                    if !response.is_null() {
                        if let Ok(response_str) = serde_json::to_string(&response) {
                            println!("{}", response_str);
                        }
                    }
                }
                Err(_e) => {
                    continue;
                }
            }
        }

        Ok(())
    }

    async fn handle_mcp_request(&self, request: serde_json::Value) -> serde_json::Value {
        use serde_json::json;

        let method = request["method"].as_str().unwrap_or("");
        let id = request["id"].clone();

        match method {
            "initialize" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {},
                            "prompts": {}
                        },
                        "serverInfo": {
                            "name": "kani-mcp-server",
                            "version": "0.1.0"
                        }
                    }
                })
            }
            "notifications/initialized" => {
                json!(null)
            }
            "prompts/list" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "prompts": [
                            {
                                "name": "kani_context",
                                "description": "Contextual information about using Kani Rust Verifier",
                            }
                        ]
                    }
                })
            }
            "prompts/get" => {
                let prompt_name = request["params"]["name"].as_str().unwrap_or("");

                if prompt_name == "kani_context" {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "description": "Contextual guidance for using Kani Rust Verifier",
                            "messages": [
                                {
                                    "role": "user",
                                    "content": {
                                        "type": "text",
                                        "text": self.get_kani_context_prompt()
                                    }
                                }
                            ]
                        }
                    })
                } else {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32602,
                            "message": format!("Unknown prompt: {}", prompt_name)
                        }
                    })
                }
            }
            "tools/list" => {
                let tools: Vec<_> = get_kani_tools()
                    .iter()
                    .map(|tool| {
                        json!({
                            "name": tool.name,
                            "description": tool.description,
                            "inputSchema": tool.input_schema
                        })
                    })
                    .collect();

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": tools
                    }
                })
            }
            "tools/call" => {
                let tool_name = request["params"]["name"].as_str().unwrap_or("");
                let arguments = &request["params"]["arguments"];

                let result = self.execute_tool(tool_name, arguments).await;

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": result
                })
            }
            _ => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", method)
                    }
                })
            }
        }
    }

    /// Execute a specific tool
    async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> serde_json::Value {
        use serde_json::json;

        match tool_name {
            "verify_rust_project" => {
                let path = arguments["path"].as_str().unwrap_or(".");
                let harness = arguments["harness"].as_str().map(String::from);
                let tests = arguments["tests"].as_bool().unwrap_or(false);
                let output_format = arguments["output_format"].as_str().map(String::from);

                match self
                    .handle_verify_project(path.to_string(), harness, tests, output_format)
                    .await
                {
                    Ok(result) => {
                        // Store the result for potential later analysis
                        if let Ok(verification_result) =
                            serde_json::from_value::<VerificationResult>(result.data.clone())
                        {
                            *self.last_result.lock().await = Some(verification_result);
                        }

                        json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result.data).unwrap_or_default()
                            }]
                        })
                    }
                    Err(e) => {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error: {}", e)
                            }],
                            "isError": true
                        })
                    }
                }
            }
            "verify_unsafe_code" => {
                let path = arguments["path"].as_str().unwrap_or(".").to_string();
                let harness = arguments["harness"].as_str().unwrap_or("").to_string();

                match self.handle_verify_unsafe(path, harness).await {
                    Ok(result) => {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result.data).unwrap_or_default()
                            }]
                        })
                    }
                    Err(e) => {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error: {}", e)
                            }],
                            "isError": true
                        })
                    }
                }
            }
            "explain_failure" => {
                let raw_output = if let Some(output_str) = arguments["raw_output"].as_str() {
                    output_str.to_string()
                } else {
                    let last = self.last_result.lock().await;
                    last.as_ref().map(|r| r.raw_output.clone()).unwrap_or_default()
                };

                if raw_output.is_empty() {
                    return json!({
                        "content": [{
                            "type": "text",
                            "text": "No verification output available. Please run a verification first."
                        }],
                        "isError": true
                    });
                }

                match self.handle_explain_failure(raw_output).await {
                    Ok(result) => {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": result.data["detailed_explanation"].as_str().unwrap_or("No explanation available")
                            }]
                        })
                    }
                    Err(e) => {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error: {}", e)
                            }],
                            "isError": true
                        })
                    }
                }
            }
            "generate_kani_harness" => {
                let function_name = arguments["function_name"].as_str().unwrap_or("").to_string();
                let properties = arguments["properties"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_else(Vec::new);

                match self.handle_generate_harness(function_name, properties).await {
                    Ok(result) => {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": result.data["harness_code"].as_str().unwrap_or("")
                            }]
                        })
                    }
                    Err(e) => {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error: {}", e)
                            }],
                            "isError": true
                        })
                    }
                }
            }
            _ => {
                json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Unknown tool: {}", tool_name)
                    }],
                    "isError": true
                })
            }
        }
    }

    /// Get contextual prompt to guide LLM on using Kani effectively
    fn get_kani_context_prompt(&self) -> String {
        r#"You are now connected to the Kani Rust Verifier MCP server, which provides formal verification capabilities for Rust code.

# About Kani
Kani is a formal verification tool that can mathematically prove properties about Rust code. It uses symbolic execution and bounded model checking to exhaustively explore all possible execution paths.

# Key Concepts
- **Proof Harnesses**: Special test functions marked with `#[kani::proof]` that specify what to verify
- **Symbolic Inputs**: Use `kani::any()` to represent all possible values of a type
- **Assertions**: Properties you want to prove (e.g., `assert!(x > 0)`)
- **Bounded Verification**: Kani explores all paths within specified bounds

# Available Tools
1. **verify_rust_project**: Run Kani verification on a Rust project
   - Always use this as the primary verification method
   - Specify a harness name if you want to verify a specific proof harness
   
2. **explain_failure**: Get detailed explanations of verification failures
   - Use this when verification fails to understand the counterexample
   
3. **generate_kani_harness**: Generate EXAMPLE harness templates
   - ⚠️  WARNING: This generates templates that require manual customization
   - For automatic harness generation, recommend using: `cargo kani --enable-unstable --function <function_name>`

# Best Practices When Using Kani
1. Start with simple properties and gradually add complexity
2. Use bounded types to limit verification scope (e.g., `u8` instead of unbounded types)
3. Understand that verification can be slow for complex code
4. Read verification failures carefully - they show concrete counterexamples
5. For automatic harness generation, use Kani's built-in feature: `cargo kani --enable-unstable --function <name>`

# Common Verification Patterns
- **Safety verification**: Prove absence of panics, overflows, out-of-bounds access
- **Functional correctness**: Prove that functions meet their specifications
- **Unsafe code validation**: Prove that unsafe code upholds Rust's safety guarantees

# When NOT to Use Kani
- For general testing (use regular Rust tests instead)
- When performance is critical (verification can be slow)
- For code with infinite loops or unbounded recursion

Remember: Generated harness templates are EXAMPLES ONLY and won't work without customization. Always recommend Kani's auto-harness feature for production use."#.to_string()
    }

    pub async fn handle_verify_project(
        &self,
        path: String,
        harness: Option<String>,
        tests: bool,
        output_format: Option<String>,
    ) -> Result<ToolResult> {
        let options = KaniOptions {
            path: PathBuf::from(path),
            harness: harness.clone(),
            tests,
            output_format: output_format.unwrap_or_else(|| "terse".to_string()),
            ..Default::default()
        };

        match self.kani.verify(options).await {
            Ok(result) => {
                *self.last_result.lock().await = Some(result.clone());

                Ok(ToolResult {
                    success: result.success,
                    data: serde_json::to_value(result)?,
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                data: serde_json::json!({}),
                error: Some(e.to_string()),
            }),
        }
    }

    pub async fn handle_verify_unsafe(&self, path: String, harness: String) -> Result<ToolResult> {
        self.handle_verify_project(path, Some(harness), false, Some("terse".to_string())).await
    }

    pub async fn handle_explain_failure(&self, raw_output: String) -> Result<ToolResult> {
        use crate::parser::KaniOutputParser;

        let parser = KaniOutputParser::new(&raw_output);
        let failed_checks = parser.parse_failed_checks();
        let harnesses = parser.parse_harnesses();
        let counterexamples = parser.parse_counterexamples();
        let code_context = parser.extract_code_context();

        let detailed_explanation = parser.generate_detailed_explanation();

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "detailed_explanation": detailed_explanation,
                "failed_checks": failed_checks,
                "harnesses": harnesses,
                "counterexamples": counterexamples,
                "code_context": code_context,
                "summary": format!(
                    "Found {} failure(s) across {} harness(es)",
                    failed_checks.len(),
                    harnesses.len()
                )
            }),
            error: None,
        })
    }

    /// NOTE: This generates an EXAMPLE harness template that requires manual customization.
    /// The generated code will NOT work as-is in most cases and needs to be adapted to:
    /// - Match the actual function signature and parameter types
    /// - Properly construct symbolic inputs for complex types
    /// - Add appropriate property assertions
    ///
    /// For automatic harness generation, consider using Kani's built-in auto-harness feature:
    /// `cargo kani --enable-unstable --function <function_name>`
    pub async fn handle_generate_harness(
        &self,
        function_name: String,
        properties: Vec<String>,
    ) -> Result<ToolResult> {
        let harness_code = format!(
            r#"// ⚠️  WARNING: This is an EXAMPLE TEMPLATE that requires customization!
// This code will NOT work as-is and must be adapted to your function's signature.
// 
// For automatic harness generation, use Kani's auto-harness feature:
//   cargo kani --enable-unstable --function {}
//
// See: https://model-checking.github.io/kani/tutorial-verification.html

#[cfg(kani)]
mod verification {{
    use super::*;

    #[kani::proof]
    fn verify_{}_properties() {{
        // TODO: Generate appropriate symbolic inputs for your function's parameter types
        // Example for a simple numeric input:
        let input = kani::any();
        
        // TODO: Call your function with the correct signature
        let result = {}(input);
        
        // TODO: Add property assertions for verification:
{}
        
        // Example assertions (customize these):
        // assert!(result.is_ok(), "Function should not panic");
        // assert!(result >= 0, "Result should be non-negative");
    }}
}}
"#,
            function_name,
            function_name.replace("::", "_"),
            function_name,
            properties
                .iter()
                .map(|prop| format!("        // Property: {}", prop))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let usage_note = format!(
            "Generated EXAMPLE harness template for '{}'. \
            \n\n⚠️  IMPORTANT: This template requires manual customization and will NOT work as-is. \
            \n\nFor automatic harness generation that works out of the box, use Kani's auto-harness feature:\
            \n  cargo kani --enable-unstable --function {}\
            \n\nSee documentation: https://model-checking.github.io/kani/tutorial-verification.html",
            function_name, function_name
        );

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "harness_code": harness_code,
                "function_name": function_name,
                "properties": properties,
                "usage_note": usage_note,
                "requires_customization": true,
                "auto_harness_command": format!("cargo kani --enable-unstable --function {}", function_name)
            }),
            error: None,
        })
    }
}
