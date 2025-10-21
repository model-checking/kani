use crate::kani_wrapper::{KaniOptions, KaniWrapper, VerificationResult};
use crate::tools::{get_kani_tools, ToolResult};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Main MCP server for Kani integration
pub struct KaniMcpServer {
    kani: Arc<KaniWrapper>,
    last_result: Arc<Mutex<Option<VerificationResult>>>,
}

impl KaniMcpServer {
    /// Create a new Kani MCP server
    pub fn new() -> Result<Self> {
        let kani = Arc::new(KaniWrapper::new()?);
        Ok(Self {
            kani,
            last_result: Arc::new(Mutex::new(None)),
        })
    }

    /// Run the MCP server
    pub async fn run(self) -> Result<()> {
        info!("═══════════════════════════════════════════════════════════");
        info!("  Kani MCP Server - Ready for Amazon Q Integration");
        info!("═══════════════════════════════════════════════════════════");
        info!("");
        info!("📋 Available Tools:");
        
        for tool in get_kani_tools() {
            info!("  • {} - {}", tool.name, tool.description);
        }
        
        info!("");
        info!("🔗 Connection: stdio (Standard Input/Output)");
        info!("🤖 Compatible with: Amazon Q, Claude Desktop, Cursor, etc.");
        info!("");
        info!("⚡ Server is ready and waiting for requests...");
        info!("   Press Ctrl+C to stop");
        info!("");

        // In a real MCP implementation, you would:
        // 1. Listen for JSON-RPC messages on stdin
        // 2. Parse the messages according to MCP protocol
        // 3. Call the appropriate handler methods
        // 4. Send responses to stdout
        //
        // For now, this is a skeleton that demonstrates the structure

        // Keep server alive
        tokio::signal::ctrl_c().await?;
        info!("");
        info!("🛑 Shutting down Kani MCP Server...");

        Ok(())
    }

    /// Handle verify_rust_project tool call
    pub async fn handle_verify_project(
        &self,
        path: String,
        harness: Option<String>,
        tests: bool,
        output_format: Option<String>,
    ) -> Result<ToolResult> {
        info!("🔍 Tool called: verify_rust_project");
        info!("   Path: {}", path);
        
        let options = KaniOptions {
            path: PathBuf::from(path),
            harness: harness.clone(),
            tests,
            output_format: output_format.unwrap_or_else(|| "terse".to_string()),
            ..Default::default()
        };

        match self.kani.verify(options).await {
            Ok(result) => {
                // Store result for later reference
                *self.last_result.lock().await = Some(result.clone());

                Ok(ToolResult {
                    success: result.success,
                    data: serde_json::to_value(result)?,
                    error: None,
                })
            }
            Err(e) => {
                error!("Verification error: {}", e);
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
        info!("🔍 Tool called: verify_unsafe_code");
        
        // This is essentially the same as verify_project but focused on unsafe code
        self.handle_verify_project(path, Some(harness), false, Some("terse".to_string())).await
    }

    /// Handle explain_kani_failure tool call
    pub async fn handle_explain_failure(&self, raw_output: String) -> Result<ToolResult> {
        info!("🔍 Tool called: explain_kani_failure");
        
        use crate::parser::KaniOutputParser;
        
        let parser = KaniOutputParser::new(&raw_output);
        let failed_checks = parser.parse_failed_checks();
        let harnesses = parser.parse_harnesses();

        let explanation = format!(
            "Kani verification failed with {} issue(s):\n\n{}",
            failed_checks.len(),
            failed_checks.iter()
                .map(|check| format!(
                    "• {} at {}:{} in {}", 
                    check.description,
                    check.file,
                    check.line.map(|l| l.to_string()).unwrap_or_else(|| "?".to_string()),
                    check.function
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "explanation": explanation,
                "failed_checks": failed_checks,
                "harnesses": harnesses,
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
        info!("🔍 Tool called: generate_kani_harness");
        
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