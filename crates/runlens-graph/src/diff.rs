use crate::graph::EventGraph;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ChangedSpan {
    pub name: String,
    pub old_duration_ms: Option<f64>,
    pub new_duration_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphDiff {
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub added_edges: Vec<String>,
    pub removed_edges: Vec<String>,
    pub changed_spans: Vec<ChangedSpan>,
}

pub fn compare(a: &EventGraph, b: &EventGraph) -> GraphDiff {
    let a_ids: std::collections::HashSet<&str> = a.nodes.iter().map(|n| n.id.as_str()).collect();
    let b_ids: std::collections::HashSet<&str> = b.nodes.iter().map(|n| n.id.as_str()).collect();

    let added_nodes: Vec<String> = b_ids.difference(&a_ids).map(|s| (*s).to_string()).collect();
    let removed_nodes: Vec<String> = a_ids.difference(&b_ids).map(|s| (*s).to_string()).collect();

    let a_edges: std::collections::HashSet<String> = a.edges.iter().map(|e| format!("{}->{}", e.from, e.to)).collect();
    let b_edges: std::collections::HashSet<String> = b.edges.iter().map(|e| format!("{}->{}", e.from, e.to)).collect();

    let added_edges: Vec<String> = b_edges.difference(&a_edges).cloned().collect();
    let removed_edges: Vec<String> = a_edges.difference(&b_edges).cloned().collect();

    let mut changed_spans = Vec::new();
    for na in &a.nodes {
        if let Some(nb) = b.nodes.iter().find(|n| n.id == na.id) {
            if na.duration_ms != nb.duration_ms {
                changed_spans.push(ChangedSpan {
                    name: na.name.clone(),
                    old_duration_ms: na.duration_ms,
                    new_duration_ms: nb.duration_ms,
                });
            }
        }
    }

    GraphDiff {
        added_nodes,
        removed_nodes,
        added_edges,
        removed_edges,
        changed_spans,
    }
}
