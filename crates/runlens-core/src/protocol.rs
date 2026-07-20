use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

pub const JSON_RPC_VERSION: &str = "2.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub error: JsonRpcError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl JsonRpcError {
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self { code: -32700, message: msg.into(), data: None }
    }
    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self { code: -32600, message: msg.into(), data: None }
    }
    pub fn method_not_found(msg: impl Into<String>) -> Self {
        Self { code: -32601, message: msg.into(), data: None }
    }
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self { code: -32602, message: msg.into(), data: None }
    }
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self { code: -32603, message: msg.into(), data: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    Num(u64),
    Str(String),
    Null,
}

impl From<JsonRpcId> for Option<String> {
    fn from(id: JsonRpcId) -> Self {
        match id {
            JsonRpcId::Num(n) => Some(n.to_string()),
            JsonRpcId::Str(s) => Some(s),
            JsonRpcId::Null => None,
        }
    }
}

pub mod methods {
    pub const SESSION_START: &str = "session.start";
    pub const SESSION_STOP: &str = "session.stop";
    pub const EVENT_EMIT: &str = "event.emit";
    pub const EVENT_EMIT_BATCH: &str = "event.emit_batch";
    pub const DAEMON_STATUS: &str = "daemon.status";
    pub const DAEMON_SHUTDOWN: &str = "daemon.shutdown";
    pub const DAEMON_INGEST: &str = "daemon.ingest";
    pub const DAEMON_SUBSCRIBE: &str = "daemon.subscribe";
    pub const EVENT_SUBSCRIBE: &str = "event.subscribe";
    pub const PROXY_START: &str = "proxy.start";
    pub const PROXY_STOP: &str = "proxy.stop";
    pub const CHECKPOINT_CREATE: &str = "checkpoint.create";
    pub const CHECKPOINT_RESTORE: &str = "checkpoint.restore";
    pub const CHECKPOINT_LIST: &str = "checkpoint.list";
    pub const MINIMIZER_START: &str = "minimizer.start";
    pub const MINIMIZER_STATUS: &str = "minimizer.status";
    pub const BISECT_START: &str = "bisect.start";
    pub const BISECT_STATUS: &str = "bisect.status";
    pub const BISECT_RESUME: &str = "bisect.resume";
    pub const GRAPH_TRACE: &str = "graph.trace";
    pub const GRAPH_CRITICAL: &str = "graph.critical";
    pub const GRAPH_COMPARE: &str = "graph.compare";
    pub const GRAPH_CHAIN: &str = "graph.chain";
    pub const QUERY_EXECUTE: &str = "query.execute";
    pub const QUERY_EXPLAIN: &str = "query.explain";
    pub const SESSION_LIST: &str = "session.list";
    pub const SESSION_GET: &str = "session.get";
    pub const RECORD_START: &str = "record.start";
    pub const RECORD_STOP: &str = "record.stop";
}

#[derive(Debug, Clone)]
pub enum IpcMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

impl IpcMessage {
    pub fn method(&self) -> &str {
        match self {
            Self::Request(r) => &r.method,
            Self::Notification(n) => &n.method,
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, JsonRpcError> {
        let parsed: Value = serde_json::from_slice(data)
            .map_err(|e| JsonRpcError::parse_error(format!("malformed json: {e}")))?;
        let obj = parsed.as_object()
            .ok_or_else(|| JsonRpcError::parse_error("body is not an object"))?;

        let rpc_ver = obj.get("jsonrpc")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if rpc_ver != JSON_RPC_VERSION && !rpc_ver.is_empty() {
            return Err(JsonRpcError::invalid_request(
                format!("unsupported jsonrpc version: {rpc_ver}"),
            ));
        }

        let method = obj.get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_request("missing method field"))?
            .to_owned();

        let params = obj.get("params").cloned();

        if obj.contains_key("id") {
            let id: JsonRpcId = serde_json::from_value(obj["id"].clone())
                .map_err(|e| JsonRpcError::parse_error(format!("bad request id: {e}")))?;
            Ok(Self::Request(JsonRpcRequest {
                jsonrpc: JSON_RPC_VERSION.into(),
                id,
                method,
                params,
            }))
        } else {
            Ok(Self::Notification(JsonRpcNotification {
                jsonrpc: JSON_RPC_VERSION.into(),
                method,
                params,
            }))
        }
    }
}

pub mod responses {
    use serde_json::Value;

    pub fn status(
        daemon_version: &str,
        pid: u64,
        uptime_secs: u64,
        active_sessions: usize,
        db_path: &str,
    ) -> Value {
        serde_json::json!({
            "version": daemon_version,
            "pid": pid,
            "uptime_secs": uptime_secs,
            "active_sessions": active_sessions,
            "db_path": db_path,
            "status": "running",
        })
    }
}
