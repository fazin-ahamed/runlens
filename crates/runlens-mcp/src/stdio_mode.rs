use std::io::{self, BufRead, Write};

use serde_json::Value;

use crate::tools::{list_tool_definitions, ToolCall};
use runlens_storage::Repository;

pub async fn run(repo: Repository) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(req) => handle(&repo, req).await,
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

async fn handle(repo: &Repository, req: Value) -> Option<Value> {
    let req_obj = req.as_object()?;
    let id = req_obj.get("id").cloned();
    let method = req_obj.get("method")?.as_str()?;
    let params = req_obj.get("params").cloned().unwrap_or(Value::Null);
    let rpc_result = match method {
        "initialize" => Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {"name":"runlens","version":env!("CARGO_PKG_VERSION")},
            "capabilities": {"tools":{"listChanged":false}}
        })),
        "tools/list" => Some(serde_json::json!({
            "tools": list_tool_definitions()
        })),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            let call_value = serde_json::json!({
                "name": name,
                "arguments": args
            });
            match serde_json::from_value::<ToolCall>(call_value) {
                Ok(tool_call) => match crate::tools::dispatch(repo, tool_call).await {
                    Ok(tool_output) => Some(serde_json::json!({
                        "content": [{"type":"text","text":serde_json::to_string_pretty(&tool_output).unwrap_or_default()}]
                    })),
                    Err(e) => Some(serde_json::json!({
                        "content": [{"type":"text","text":format!("error: {e}")}],
                        "isError": true
                    })),
                },
                Err(e) => Some(serde_json::json!({
                    "content": [{"type":"text","text":format!("invalid tool call: {e}")}],
                    "isError": true
                })),
            }
        },
        _ => None,
    };
    match rpc_result {
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
