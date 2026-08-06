use runlens_core::model::Event;
use runlens_storage::Repository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub source: Option<String>,
    pub duration_ms: Option<f64>,
    pub timestamp: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub struct GraphBuilder<'a> {
    repo: &'a Repository,
}

impl<'a> GraphBuilder<'a> {
    pub fn new(repo: &'a Repository) -> Self {
        Self { repo }
    }

    pub fn load(&self, trace_id: &str) -> anyhow::Result<EventGraph> {
        let events = self.repo.list_events(trace_id)?;
        build_graph_from_events(&events)
    }

    pub fn load_session(&self, session_id: &str) -> anyhow::Result<EventGraph> {
        let events = self.repo.list_events(session_id)?;
        build_graph_from_events(&events)
    }
}

fn build_graph_from_events(events: &[Event]) -> anyhow::Result<EventGraph> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_map: HashMap<&str, usize> = HashMap::new();

    for event in events {
        let idx = nodes.len();
        node_map.insert(&event.event_id, idx);
        nodes.push(GraphNode {
            id: event.event_id.clone(),
            name: event.kind.clone(),
            source: Some(event.source.to_string()),
            duration_ms: event.duration_ns.map(|ns| ns as f64 / 1_000_000.0),
            timestamp: event.utc_timestamp.to_rfc3339(),
            kind: event.kind.clone(),
        });

        if let Some(parent_id) = &event.parent_event_id {
            if node_map.contains_key(parent_id.as_str()) {
                edges.push(GraphEdge {
                    from: parent_id.clone(),
                    to: event.event_id.clone(),
                    kind: "parent".into(),
                    label: None,
                });
            }
        }
    }

    Ok(EventGraph { nodes, edges })
}
