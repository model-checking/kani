mod kani_wrapper;
mod mcp_server;
mod parser;
mod tools;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("kani_mcp_server=info".parse()?)
        )
        .init();

    println!("🚀 Kani MCP Server - Model Context Protocol for Kani Rust Verifier");
    println!("📋 Purpose: Enable AI assistants like Amazon Q to run Kani verification");
    println!();

    // Create and run server
    let server = mcp_server::KaniMcpServer::new()?;
    server.run().await?;

    Ok(())
}