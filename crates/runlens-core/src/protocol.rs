use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

pub const JSON_RPC_VERSION: &str = "2.0";

/// JSON-RPC 2.0 request with optional id (notifications have no id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 notification (request without id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response (success).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub result: Value,
}

/// JSON-RPC 2.0 error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub error: JsonRpcError,
}

/// JSON-RPC 2.0 error object.
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

/// Request or notification id.
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

/// IPC method constants for daemon communication.
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
}

/// A parsed IPC message from the wire.
#[derive(Debug,