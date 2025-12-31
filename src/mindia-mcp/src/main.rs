//! Mindia MCP Server
//!
//! Model Context Protocol server for Mindia API
//! Run with: MINDIA_API_KEY=xxx MINDIA_API_URL=xxx mindia-mcp

use anyhow::Context;
use mindia_mcp::{api_client::ApiClient, McpServer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Create API client
    let api_client = ApiClient::from_env().context(
        "Failed to create API client. Set MINDIA_API_KEY and MINDIA_API_URL environment variables",
    )?;

    // Create and run MCP server
    let mut server = McpServer::new(api_client);
    server.run().await.context("MCP server error")?;

    Ok(())
}
