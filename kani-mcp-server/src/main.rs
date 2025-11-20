mod kani_wrapper;
mod mcp_server;
mod parser;
mod tools;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Only initialize logging if explicitly requested via environment variable
    if std::env::var("KANI_MCP_LOG").is_ok() {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env().add_directive("kani_mcp_server=info".parse()?),
            )
            .init();
    }

    // Banner - only show if logging is enabled
    if std::env::var("KANI_MCP_LOG").is_ok() {
        eprintln!("Kani MCP Server - Model Context Protocol for Kani Rust Verifier");
        eprintln!("Purpose: Enable AI assistants like Amazon Q to run Kani verification");
        eprintln!();
    }

    // Create and run server
    let server = mcp_server::KaniMcpServer::new()?;
    server.run().await?;

    Ok(())
}
