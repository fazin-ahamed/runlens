//! MCP tools registry — safe, read-only surface for AI assistants.
//!
//! Every tool must:
//!  - Take a fully-validated JSON input
//!  - Return a structured JSON content
//!  - Refuse on validation failure (no silent defaults)
//!
//! Currently available:
//!  - `runlens.list_sessions` — list recent sessions for the active project.
//!  - `runlens.get_session` — get a session by id, with hash head.
//!  - `runlens.find_errors` — surface error-shaped events with sequencing.
//!  - `runlens.compare_sessions` — explainable divergence surface.
//!  - `runlens.redactions` — list redaction findings for a session.
//!  - `runlens.verify_session` — recompute chain and return the verdict.

use runlens_core::chain;
use runlens_core::compare::compare_sessions;
use runlens_storage::Repository;

pub fn list_tool_definitions() -> Vec<crate::ToolDefinition> {
    vec![
        tool_def(
            "runlens.list_sessions",
            "List recent RunLens sessions. Inputs: { limit?: number }.",
            json_schema_object(&[("limit", json_schema_int())]),
        ),
        tool_def(
            "runlens.get_session",
            "Get a session summary + integrity status. Inputs: { session_id: string }.",
            json_schema_object(&[("session_id", json_schema_string())]),
        ),
        tool_def(
            "runlens.find_errors",
            "Find error-shaped events in a session. Inputs: { session_id: string }.",
            json_schema_object(&[("session_id", json_schema_string())]),
        ),
        tool_def(
            "runlens.compare_sessions",
            "Compare two sessions and return explainable divergences.",
            json_schema_object(&[
                ("baseline", json_schema_string()),
                ("candidate", json_schema_string()),
            ]),
        ),
        tool_def(
            "runlens.redactions",
            "List the redaction findings recorded for a session.",
            json_schema_object(&[("session_id", json_schema_string())]),
        ),
        tool_def(
            "runlens.verify_session",
            "Recompute the recorded hash chain and report pass/fail.",
            json_schema_object(&[("session_id", json_schema_string())]),
        ),
    ]
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "name", content = "arguments")]
pub enum ToolCall {
    #[serde(rename = "runlens.list_sessions")]
    ListSessions { arguments: ListArgs },
    #[serde(rename = "runlens.get_session")]
    GetSession { arguments: SessionArg },
    #[serde(rename = "runlens.find_errors")]
    FindErrors { arguments: SessionArg },
    #[serde(rename = "runlens.compare_sessions")]
    CompareSessions { arguments: CompareArg },
    #[serde(rename = "runlens.redactions")]
    Redactions { arguments: SessionArg },
    #[serde(rename = "runlens.verify_session")]
    VerifySession { arguments: SessionArg },
}

#[derive(Debug, serde::Deserialize)]
pub struct ListArgs {
    pub limit: Option<u32>,
}
#[derive(Debug, serde::Deserialize)]
pub struct SessionArg {
    pub session_id: String,
}
#[derive(Debug, serde::Deserialize)]
pub struct CompareArg {
    pub baseline: String,
    pub candidate: String,
}

pub async fn dispatch(repo: &Repository, call: ToolCall) -> anyhow::Result<serde_json::Value> {
    match call {
        ToolCall::ListSessions { arguments } => {
            let limit = arguments.limit.unwrap_or(20);
            let sessions = repo.list_recent_sessions(limit).await?;
            let rows: Vec<_> = sessions
                .into_iter()
                .map(|s| {
                    serde_json::json!({
                        "session_id": s.session_id,
                        "project_id": s.project_id,
                        "state": s.state.as_str(),
                        "started_at": s.started_at.to_rfc3339(),
                        "stopped_at": s.stopped_at.map(|t| t.to_rfc3339()),
                        "events": s.source_event_count,
                        "command": s.command,
                    })
                })
                .collect();
            Ok(serde_json::json!({ "sessions": rows }))
        }
        ToolCall::GetSession { arguments } => {
            let s = repo
                .get_session(&arguments.session_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("session not found"))?;
            let events = repo.list_events(&s.session_id).await?;
            let head = events.last().and_then(|e| e.current_hash.clone());
            let verify = chain::verify_chain(&events).is_ok();
            Ok(serde_json::json!({
                "session": {
                    "session_id": s.session_id,
                    "project_id": s.project_id,
                    "state": s.state.as_str(),
                    "started_at": s.started_at.to_rfc3339(),
                    "stopped_at": s.stopped_at.map(|t| t.to_rfc3339()),
                    "events": events.len(),
                    "command": s.command,
                    "args": s.args,
                    "labels": s.labels,
                    "head_hash": head,
                    "chain_valid": verify,
                }
            }))
        }
        ToolCall::FindErrors { arguments } => {
            let events = repo.list_events(&arguments.session_id).await?;
            let errors: Vec<_> = events
                .iter()
                .filter(|e| e.is_error_like())
                .map(|e| {
                    serde_json::json!({
                        "sequence": e.sequence,
                        "kind": e.kind,
                        "severity": e.severity.as_str(),
                        "ts": e.utc_timestamp.to_rfc3339(),
                        "hash": e.current_hash,
                        "payload": e.payload,
                    })
                })
                .collect();
            Ok(serde_json::json!({ "errors": errors }))
        }
        ToolCall::CompareSessions { arguments } => {
            let a = repo.list_events(&arguments.baseline).await?;
            let b = repo.list_events(&arguments.candidate).await?;
            let cmp = compare_sessions(&a, &b);
            Ok(serde_json::to_value(&cmp).unwrap_or_default())
        }
        ToolCall::Redactions { arguments } => {
            let rows = repo.list_redactions(&arguments.session_id).await?;
            Ok(serde_json::json!({ "redactions": rows }))
        }
        ToolCall::VerifySession { arguments } => {
            let events = repo.list_events(&arguments.session_id).await?;
            let status = chain::verify_chain(&events).is_ok();
            Ok(serde_json::json!({
                "session_id": arguments.session_id,
                "events": events.len(),
                "chain_valid": status,
            }))
        }
    }
}

fn tool_def(name: &'static str, description: &'static str, input_schema: serde_json::Value) -> crate::ToolDefinition {
    crate::ToolDefinition { name, description, input_schema }
}

fn json_schema_object(props: &[(&str, serde_json::Value)]) -> serde_json::Value {
    let mut required = Vec::new();
    let mut properties = serde_json::Map::new();
    for (name, schema) in props {
        if !schema.get("optional").and_then(|v| v.as_bool()).unwrap_or(false) {
            required.push(*name);
        }
        let mut clean = schema.clone();
        clean.as_object_mut().map(|o| o.remove("optional"));
        properties.insert((*name).to_string(), clean);
    }
    serde_json::json!({
        "type": "object",
        "properties": Value(properties),
        "required": required,
    })
}

fn json_schema_string() -> serde_json::Value {
    serde_json::json!({"type":"string"})
}

fn json_schema_int() -> serde_json::Value {
    serde_json::json!({"type":"integer"})
}

struct Value(serde_json::Map<String, serde_json::Value>);
impl serde::Serialize for Value {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}
