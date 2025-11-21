use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Definition of an MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Get all available Kani MCP tools
pub fn get_kani_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "verify_rust_project".to_string(),
            description: "Run Kani verification on a Rust project to prove safety properties and check for undefined behavior. Kani uses formal verification to mathematically prove correctness.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the Rust project directory (must contain Cargo.toml)"
                    },
                    "harness": {
                        "type": "string",
                        "description": "Optional: Specific proof harness to verify (e.g., 'module::verify::check_bounds')"
                    },
                    "tests": {
                        "type": "boolean",
                        "description": "If true, verify all #[test] functions as proof harnesses",
                        "default": false
                    },
                    "output_format": {
                        "type": "string",
                        "enum": ["regular", "terse", "old"],
                        "default": "terse",
                        "description": "Output format for verification results"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "verify_unsafe_code".to_string(),
            description: "Specifically verify unsafe Rust code blocks for memory safety violations including null pointer dereferences, buffer overflows, and use-after-free bugs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to Rust project containing unsafe code"
                    },
                    "harness": {
                        "type": "string",
                        "description": "Harness function that tests the unsafe code"
                    }
                },
                "required": ["path", "harness"]
            }),
        },
        ToolDefinition {
            name: "explain_failure".to_string(),
            description: "Analyze and explain why a Kani verification failed, providing details about counterexamples and suggested fixes.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "raw_output": {
                        "type": "string",
                        "description": "Raw Kani verification output to analyze"
                    }
                },
                "required": ["raw_output"]
            }),
        },
        ToolDefinition {
            name: "generate_kani_harness".to_string(),
            description: "Generate a Kani proof harness template for verifying a specific Rust function.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "function_name": {
                        "type": "string",
                        "description": "Name of the function to verify"
                    },
                    "properties": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of properties to verify (e.g., 'no_panic', 'bounds_check', 'overflow_check')"
                    }
                },
                "required": ["function_name"]
            }),
        },
    ]
}

/// Tool handler results
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: Value,
    pub error: Option<String>,
}
