//! Mindia MCP Server
//!
//! Model Context Protocol server that exposes Mindia API capabilities
//! as tools for AI assistants (Claude Desktop, Cursor, etc.)

pub mod api_client;
pub mod server;
pub mod tools;

pub use server::McpServer;
