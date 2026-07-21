//! MCP loopback HTTP transport.
//!
//! Tiny axum-based server. Listens on `127.0.0.1:<port>` only and
//! exposes two routes:
//!  - `POST /mcp` — a full JSON-RPC 2.0 message body
//!  - `GET  /tools` — tools/list, returns the schema catalogue
//!
//! This transport is intended for browser-based inspect tools, debug
//! proxies (mitmproxy-style), and Claude Code/Continue-style hooks that
//! prefer HTTP over stdio.

use axum::{extract::Json, http::StatusCode, response::IntoResponse, routing::{get, post}, Router};
use serde_json::Value;
use tracing::warn;

use runlens_storage::Repository;

pub async fn run(repo: Repository, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/mcp", post({
            let repo = repo.clone();
            move |Json(req): Json<Value>| async move { handle_rpc(&repo, req).await }
        }))
        .route("/tools", get({
            let _repo = repo.clone();
            move || async {
                Json(serde_json::json!({ "tools": crate::tools::list_tool_definitions() }))
            }
        }));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    warn!(?port, "runlens mcp listening loopback");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_rpc(repo: &Repository, req: Value) -> impl IntoResponse {
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let id = req.get("id").cloned();
    let result = match method {
        "initialize" => serde_json::json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {"name":"runlens","version":env!("CARGO_PKG_VERSION")},
            "capabilities": {"tools":{"listChanged":false}}
        }),
        "tools/list" => serde_json::json!({
            "tools": crate::tools::list_tool_definitions()
        }),
        "tools/call" => match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => match serde_json::from_value::<crate::tools::ToolCall>(serde_json::json!({
                "name": name,
                "arguments": params.get("arguments").cloned().unwrap_or(Value::Null)
            })) {
                Ok(call) => match crate::tools::dispatch(repo, call).await {
                    Ok(v) => serde_json::json!({
                        "content": [{"type":"text","text": v.to_string()}]
                    }),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": {"code":-32603,"message":e.to_string()}
                            })),
                        );
                    }
                },
                Err(e) => serde_json::json!({
                    "error": {"code":-32602,"message":e.to_string()}
                }),
            },
            None => serde_json::json!({
                "error": {"code":-32602,"message":"missing tool name"}
            }),
        },
        _ => serde_json::json!({
            "error": {"code":-32601,"message":format!("method not found: {method}")}
        }),
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({ "jsonrpc":"2.0","id":id,"result":result })),
    )
}
