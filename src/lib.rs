use serde::{Deserialize, Serialize};
use serde_json::json;

wit_bindgen::generate!({
    world: "mcp",
    generate_all,
});
use exports::wasmcloud::mcp::mcp_handler::Guest as Server;

struct Component;

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

impl Server for Component {
    fn mcp_handle(
        request: crate::wasi::http::types::IncomingRequest,
        response_out: crate::wasi::http::types::ResponseOutparam,
    ) {
        // --- 1. Read entire HTTP body ---
        let body_stream = request.consume().expect("failed to get body stream");
        let input_stream = body_stream.stream().expect("failed to get input stream");
        let body = {
            let mut buf = Vec::new();
            loop {
                match input_stream.blocking_read(1024 * 1024) {
                    Ok(chunk) => {
                        if chunk.is_empty() {
                            break;
                        }
                        buf.extend_from_slice(&chunk);
                    }
                    Err(_) => break,
                }
            }
            buf
        };
        let body_str = String::from_utf8_lossy(&body);

        // --- 2. Parse JSON body into MCP request ---
        let mcp: McpRequest = match serde_json::from_str(&body_str) {
            Ok(v) => v,
            Err(e) => {
                send_error_response(response_out, 400, format!("invalid json: {e}"));
                return;
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
        send_success_response(response_out, json_string);
    }
}

// Helper functions to send HTTP responses
fn send_success_response(response_out: crate::wasi::http::types::ResponseOutparam, body: String) {
    use crate::wasi::http::types::{Fields, OutgoingBody, OutgoingResponse};

    let headers = Fields::new();
    let _ = headers.set(&"content-type".to_string(), &[b"application/json".to_vec()]);

    let response = OutgoingResponse::new(headers);
    response.set_status_code(200).unwrap();

    let response_body = response.body().unwrap();
    crate::wasi::http::types::ResponseOutparam::set(response_out, Ok(response));

    let output_stream = response_body.write().unwrap();
    output_stream
        .blocking_write_and_flush(body.as_bytes())
        .unwrap();

    drop(output_stream);
    OutgoingBody::finish(response_body, None).unwrap();
}

fn send_error_response(
    response_out: crate::wasi::http::types::ResponseOutparam,
    status: u16,
    body: String,
) {
    use crate::wasi::http::types::{Fields, OutgoingBody, OutgoingResponse};

    let headers = Fields::new();

    let response = OutgoingResponse::new(headers);
    response.set_status_code(status).unwrap();

    let response_body = response.body().unwrap();
    crate::wasi::http::types::ResponseOutparam::set(response_out, Ok(response));

    let output_stream = response_body.write().unwrap();
    output_stream
        .blocking_write_and_flush(body.as_bytes())
        .unwrap();

    drop(output_stream);
    OutgoingBody::finish(response_body, None).unwrap();
}

// -------- MCP method impls ----------

fn handle_initialize(params: &serde_json::Value) -> serde_json::Value {
    let protocol_version = params
        .get("protocolVersion")
        .cloned()
        .unwrap_or(json!("2024-11-05"));
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

export!(Component);
