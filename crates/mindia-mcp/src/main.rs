//! Mindia MCP Server
//!
//! Model Context Protocol server for Mindia API
//! Run with: MINDIA_API_KEY=xxx MINDIA_API_URL=xxx mindia-mcp

use anyhow::Context;
use mindia_mcp::{ApiClient, MindiaService};
use rmcp::service::ServiceExt;
use rmcp::transport::io::stdio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let api_client = ApiClient::from_env().context(
        "Failed to create API client. Set MINDIA_API_KEY and MINDIA_API_URL environment variables",
    )?;

    let service = MindiaService::new(api_client);
    let running = service.serve(stdio()).await.context("MCP transport failed")?;
    running.waiting().await.context("MCP server error")?;

    Ok(())
}
