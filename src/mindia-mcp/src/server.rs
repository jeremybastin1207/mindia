//! MCP Server implementation
//!
//! Implements the Model Context Protocol (MCP) to expose Mindia tools
//! to AI assistants via JSON-RPC over stdio

use crate::api_client::ApiClient;
use crate::tools::ToolRegistry;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Write};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum McpRequest {
    Initialize {
        params: InitializeParams,
    },
    ToolsList {
        #[serde(skip)]
        id: Option<serde_json::Value>,
    },
    ToolsCall {
        name: String,
        arguments: serde_json::Value,
        #[serde(skip)]
        id: Option<serde_json::Value>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    protocol_version: String,
    capabilities: serde_json::Value,
    client_info: ClientInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(flatten)]
    result: McpResponseResult,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpResponseResult {
    Success {
        #[serde(flatten)]
        data: serde_json::Value,
    },
    Error {
        error: McpError,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

pub struct McpServer {
    api_client: ApiClient,
    tool_registry: ToolRegistry,
}

impl McpServer {
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client,
            tool_registry: ToolRegistry::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader
                .read_line(&mut line)
                .context("Failed to read from stdin")?;

            if bytes_read == 0 {
                break; // EOF
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match self.handle_request(trimmed).await {
                Ok(Some(response)) => {
                    let output =
                        serde_json::to_string(&response).context("Failed to serialize response")?;
                    println!("{}", output);
                    io::stdout().flush().context("Failed to flush stdout")?;
                }
                Ok(None) => {
                    // No response needed
                }
                Err(e) => {
                    let error_response = McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: McpResponseResult::Error {
                            error: McpError {
                                code: -32603,
                                message: format!("Internal error: {}", e),
                                data: None,
                            },
                        },
                    };
                    let output = serde_json::to_string(&error_response)
                        .context("Failed to serialize error response")?;
                    eprintln!("{}", output);
                    io::stderr().flush().context("Failed to flush stderr")?;
                }
            }
        }

        Ok(())
    }

    async fn handle_request(&mut self, line: &str) -> Result<Option<McpResponse>> {
        let request: serde_json::Value =
            serde_json::from_str(line).context("Failed to parse JSON request")?;

        let id = request.get("id").cloned();
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .context("Missing method field")?;

        match method {
            "initialize" => {
                let _params: InitializeParams =
                    serde_json::from_value(request.get("params").cloned().unwrap_or_default())
                        .context("Invalid initialize params")?;

                let response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: id.clone(),
                    result: McpResponseResult::Success {
                        data: serde_json::json!({
                            "protocolVersion": "2024-11-05",
                            "capabilities": {
                                "tools": {}
                            },
                            "serverInfo": {
                                "name": "mindia-mcp",
                                "version": "0.1.0"
                            }
                        }),
                    },
                };
                Ok(Some(response))
            }
            "tools/list" => {
                let tools = self.tool_registry.list_tools();
                let response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: id.clone(),
                    result: McpResponseResult::Success {
                        data: serde_json::json!({
                            "tools": tools
                        }),
                    },
                };
                Ok(Some(response))
            }
            "tools/call" => {
                let name = request
                    .get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .context("Missing tool name")?;

                let arguments = request
                    .get("params")
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or_default();

                let result = self
                    .tool_registry
                    .call_tool(name, arguments, &self.api_client)
                    .await?;

                let response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: id.clone(),
                    result: McpResponseResult::Success {
                        data: serde_json::json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string(&result)?
                                }
                            ]
                        }),
                    },
                };
                Ok(Some(response))
            }
            _ => {
                let response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: id.clone(),
                    result: McpResponseResult::Error {
                        error: McpError {
                            code: -32601,
                            message: format!("Method not found: {}", method),
                            data: None,
                        },
                    },
                };
                Ok(Some(response))
            }
        }
    }
}
