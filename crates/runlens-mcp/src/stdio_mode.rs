//! MCP stdio transport.
//!
//! Implementation is a minimal hand-rolled JSON-RPC 2.0 server over
//! newline-delimited JSON on stdin/stdout. We avoid pulling in the
//! official MCP SDK to keep this crate dependency-light and let the
//! surface be auditable by humans.

use std::io::{self, BufRead, Write};

use serde_json::Value;
use tracing::warn;

use runlens_storage::Repository;

pub async fn run(repo: Repository) -> anyhow::Result<()> {
    let _ = repo; // not used in this minimal scaffold.
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let response: Option<Value> = match serde_json::from_str::<Value>(&line) {
            Ok(req) => handle(req),
            Err(e) => Some(error_response(None, -32700, format!("parse error: {e}"))),
        };
        if let Some(resp) = response {
            let payload = resp.to_string();
            stdout.write_all(payload.as_bytes())?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn handle(req: Value) -> Option<Value> {
    let req_obj = req.as_object()?;
    let id = req_obj.get("id").cloned();
    let method = req_obj.get("method")?.as_str()?;
    let params = req_obj.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {"name":"runlens","version":env!("CARGO_PKG_VERSION")},
            "capabilities": {"tools":{"listChanged":false}}
        })),
        "tools/list" => Some(serde_json::json!({
            "tools": crate::tools::list_tool_definitions()
        })),
        "tools/call" => {
            // We do not actually call into repo here for stdio scaffold.
            // The HTTP transport wires the real tool dispatch.
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            warn!(%name, ?args, "deferred tool call (use --http-port)");
            Some(serde_json::json!({
                "content": [{"type":"text","text":"deferred"}]
            }))
        }
        _ => None,
    };
    match result {
        Some(r) => Some(serde_json::json!({"jsonrpc":"2.0","id":id,"result":r})),
        None => Some(error_response(id, -32601, format!("method not found: {method}"))),
    }
}

fn error_response(id: Option<Value>, code: i32, message: String) -> Value {
    serde_json::json!({
        "jsonrpc":"2.0",
        "id": id,
        "error": {"code":code,"message":message}
    })
}
