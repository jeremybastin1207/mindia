# MCP Server (Model Context Protocol)

The **mindia-mcp** binary is a stdio MCP server that exposes Mindia API operations as tools for AI assistants (Cursor, Claude Desktop, etc.), built with the [rmcp](https://docs.rs/rmcp) SDK.

## Requirements

- **MINDIA_API_KEY** – API key for the Mindia API (required).
- **MINDIA_API_URL** – Base URL of the Mindia API (default: `http://localhost:3000`).  
  You can use **API_URL** instead.

## Running the server

```bash
export MINDIA_API_KEY=your-api-key
export MINDIA_API_URL=https://your-mindia-api.example.com
cargo run -p mindia-mcp
```

Or from the workspace root after building:

```bash
MINDIA_API_KEY=your-api-key MINDIA_API_URL=https://your-mindia-api.example.com mindia-mcp
```

## Installing for Cursor

1. Build and install the binary:
   ```bash
   cargo install --path crates/mindia-mcp
   ```
2. Add the server to `~/.cursor/mcp.json` (or your project’s `.cursor/mcp.json`):
   ```json
   {
     "mcpServers": {
       "Mindia": {
         "command": "mindia-mcp",
         "env": {
           "MINDIA_API_KEY": "your-api-key",
           "MINDIA_API_URL": "https://your-mindia-api.example.com"
         }
       }
     }
   }
   ```

For local development without installing, you can run from the repo:

```json
{
  "mcpServers": {
    "Mindia (Dev)": {
      "command": "cargo",
      "args": ["run", "--manifest-path", "path/to/mindia/crates/mindia-mcp/Cargo.toml"],
      "env": {
        "MINDIA_API_KEY": "your-api-key",
        "MINDIA_API_URL": "http://localhost:3000"
      }
    }
  }
}
```

## Tools

The server exposes these tools:

| Tool | Description |
|------|-------------|
| **upload_image** | Upload an image from a local path or URL. |
| **list_media** | List media (images, videos, audio, documents) with optional type, limit, offset. |
| **get_media** | Get metadata for a media item by ID. |
| **transform_image** | Generate a transformed image URL (resize). |
| **search_media** | Semantic search over media. |
| **delete_media** | Delete a media item by ID. |
| **create_folder** | Create a folder (optional parent). |

All tools use the API key from the environment; the server talks to the Mindia API over HTTP.

## Usage

Once the MCP server is configured in Cursor (or another MCP client), you can ask the assistant to use Mindia tools in natural language. The assistant will call the appropriate tool and show you the result.

### Example prompts (Cursor / AI assistant)

- **Upload an image:**  
  “Upload the image at `./screenshot.png` to Mindia” or “Upload this image URL to Mindia: https://example.com/photo.jpg”

- **List media:**  
  “List my images in Mindia” or “Show the last 10 videos in Mindia”

- **Get details:**  
  “Get metadata for Mindia media ID …” (paste a UUID)

- **Transform an image:**  
  “Give me a 400x300 resize URL for Mindia image ID …”

- **Search:**  
  “Search my Mindia library for sunset photos” or “Find documents about invoices”

- **Delete:**  
  “Delete the Mindia media with ID …”

- **Organize:**  
  “Create a Mindia folder called ‘Q1 assets’” or “Create a subfolder ‘thumbnails’ under folder …”

### Tips

- Ensure **MINDIA_API_KEY** and **MINDIA_API_URL** are set for the process that runs the MCP server (e.g. in Cursor’s MCP `env` or in your shell before starting the server).
- Use the same API base URL and key you use for the Mindia API (e.g. in Bruno or your app).
- If a tool fails, check the API is reachable and the key has the right permissions.
