use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Read;
use wasmcloud_component::http;

struct Component;

http::export!(Component);

// ---------- MCP request structs ----------
#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
    #[serde(default)]
    id: Option<serde_json::Value>,
}

// ---------- MCP response ----------
#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: &'static str,
    result: serde_json::Value,
    id: Option<serde_json::Value>,
}

impl http::Server for Component {
    fn handle(
        request: http::IncomingRequest,
    ) -> http::Result<http::Response<impl http::OutgoingBody>> {
        // --- 1. Read entire HTTP body ---
        let (_parts, mut body) = request.into_parts();
        let mut body_bytes = Vec::new();
        body.read_to_end(&mut body_bytes).unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);

        // --- 2. Parse JSON body into MCP request ---
        let mcp: McpRequest = match serde_json::from_str(&body_str) {
            Ok(v) => v,
            Err(e) => {
                return Ok(http::Response::builder()
                    .status(400)
                    .body(format!("invalid json: {e}"))
                    .unwrap());
            }
        };

        // Route based on `method`
        let result = match mcp.method.as_str() {
            "initialize" => json!({"result": handle_initialize(&mcp.params)}),
            "tools/list" => json!({"result": handle_tools_list(&mcp.params)}),
            _ => json!({
                "error": {
                    "code": -32601,
                    "message": "Unknown method"
                }
            }),
        };

        // Prepare MCP JSON-RPC response
        let response = McpResponse {
            jsonrpc: "2.0",
            result,
            id: mcp.id,
        };

        let json_string = serde_json::to_string(&response).unwrap();

        Ok(http::Response::builder()
            .header("content-type", "application/json")
            .body(json_string)
            .unwrap())
    }
}

// -------- MCP method impls ----------

fn handle_initialize(params: &serde_json::Value) -> serde_json::Value {
    let protocol_version = params.get("protocolVersion").cloned().unwrap_or(json!("2024-11-05"));
    let capabilities = params.get("capabilities").cloned().unwrap_or(json!({}));

    json!({
        "protocolVersion": protocol_version,
        "capabilities": capabilities,
        "serverInfo": {
            "name": "rust-mcp-server",
            "version": "0.1.0"
        }
    })
}


fn handle_tools_list(_params: &serde_json::Value) -> serde_json::Value {
    json!({
        "tools": [
            json!({})
        ]
    })
}
