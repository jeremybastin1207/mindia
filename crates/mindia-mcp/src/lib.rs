//! Mindia MCP Server
//!
//! Model Context Protocol server that exposes Mindia API capabilities
//! as tools for AI assistants (Claude Desktop, Cursor, etc.)

pub mod server;
pub mod tools;

pub use mindia_api_client::ApiClient;
pub use server::MindiaService;
