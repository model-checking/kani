use crate::kani_wrapper::{KaniOptions, KaniWrapper, VerificationResult};
use crate::tools::{get_kani_tools, ToolResult};
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
        Ok(Self {
            kani,
            last_result: Arc::new(Mutex::new(None)),
        })
    }

    /// Run the MCP server
    pub async fn run(self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader, stdin};
        
        let stdin = stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }

            // Parse JSON-RPC request
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
                    // Silently ignore parse errors to avoid broken pipe
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Handle MCP protocol requests
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
                            "tools": {}
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
    async fn execute_tool(&self, tool_name: &str, arguments: &serde_json::Value) -> serde_json::Value {
        use serde_json::json;

        match tool_name {
            "verify_rust_project" => {
                let path = arguments["path"].as_str().unwrap_or(".");
                let harness = arguments["harness"].as_str().map(String::from);
                let tests = arguments["tests"].as_bool().unwrap_or(false);
                let output_format = arguments["output_format"].as_str().map(String::from);

                match self.handle_verify_project(
                    path.to_string(),
                    harness,
                    tests,
                    output_format,
                ).await {
                    Ok(result) => {
                        // Store the result for potential later analysis
                        if let Ok(verification_result) = serde_json::from_value::<VerificationResult>(result.data.clone()) {
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
                // Try to get raw output from arguments or from last result
                let raw_output = if let Some(output_str) = arguments["raw_output"].as_str() {
                    output_str.to_string()
                } else {
                    let last = self.last_result.lock().await;
                    last.as_ref()
                        .map(|r| r.raw_output.clone())
                        .unwrap_or_default()
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

    /// Handle verify_rust_project tool call
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
            Err(e) => {
                Ok(ToolResult {
                    success: false,
                    data: serde_json::json!({}),
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Handle verify_unsafe_code tool call
    pub async fn handle_verify_unsafe(
        &self,
        path: String,
        harness: String,
    ) -> Result<ToolResult> {
        self.handle_verify_project(path, Some(harness), false, Some("terse".to_string())).await
    }

    /// Handle explain_kani_failure tool call with enhanced analysis
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

    /// Handle generate_kani_harness tool call
    pub async fn handle_generate_harness(
        &self,
        function_name: String,
        properties: Vec<String>,
    ) -> Result<ToolResult> {
        let harness_code = format!(
r#"#[cfg(kani)]
mod verification {{
    use super::*;

    #[kani::proof]
    fn verify_{}_properties() {{
        // Generate symbolic inputs
        let input = kani::any();
        
        // Call the function under test
        let result = {}(input);
        
        // Verify properties:
{}
    }}
}}
"#,
            function_name.replace("::", "_"),
            function_name,
            properties.iter()
                .map(|prop| format!("        // TODO: Add assertion for: {}", prop))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "harness_code": harness_code,
                "function_name": function_name,
                "properties": properties,
            }),
            error: None,
        })
    }
}