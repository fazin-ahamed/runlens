#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceNode {
    pub id: String,
    pub kind: ProvenanceNodeKind,
    pub label: String,
    pub timestamp_ns: i64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ProvenanceNodeKind {
    DataSource,
    Computation,
    DataResult,
    Event,
    Decision,
    Artifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEdge {
    pub from: String,
    pub to: String,
    pub kind: ProvenanceEdgeKind,
    pub timestamp_ns: i64,
    pub weight: f64,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProvenanceEdgeKind {
    DerivedFrom,
    Caused,
    Influenced,
    Precondition,
    Transformed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lineage {
    pub source_id: String,
    pub target_id: String,
    pub path: Vec<String>,
    pub edges: Vec<ProvenanceEdge>,
    pub total_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalInfluence {
    pub cause_id: String,
    pub effect_id: String,
    pub confidence: f64,
    pub path_length: usize,
    pub edge_kinds: Vec<ProvenanceEdgeKind>,
}

pub struct ProvenanceGraph {
    nodes: HashMap<String, ProvenanceNode>,
    edges: Vec<ProvenanceEdge>,
    outgoing: HashMap<String, Vec<usize>>,
    incoming: HashMap<String, Vec<usize>>,
}

impl ProvenanceGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: ProvenanceNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: ProvenanceEdge) {
        let idx = self.edges.len();
        self.edges.push(edge.clone());
        self.outgoing.entry(edge.from.clone()).or_default().push(idx);
        self.incoming.entry(edge.to.clone()).or_default().push(idx);
    }

    pub fn get_node(&self, id: &str) -> Option<&ProvenanceNode> {
        self.nodes.get(id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &ProvenanceNode> {
        self.nodes.values()
    }

    pub fn edges(&self) -> &[ProvenanceEdge] {
        &self.edges
    }

    pub fn outgoing_edges(&self, node_id: &str) -> Vec<&ProvenanceEdge> {
        self.outgoing.get(node_id).map_or(vec![], |indices| {
            indices.iter().filter_map(|&i| self.edges.get(i)).collect()
        })
    }

    pub fn incoming_edges(&self, node_id: &str) -> Vec<&ProvenanceEdge> {
        self.incoming.get(node_id).map_or(vec![], |indices| {
            indices.iter().filter_map(|&i| self.edges.get(i)).collect()
        })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn trace_lineage(&self, source_id: &str, target_id: &str) -> Option<Lineage> {
        let mut parent: HashMap<String, String> = HashMap::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(source_id.to_owned());
        visited.insert(source_id.to_owned());

        while let Some(current) = queue.pop_front() {
            if current == target_id {
                let path = Self::reconstruct_path(&parent, source_id, target_id);
                let edges = path
                    .windows(2)
                    .filter_map(|w| self.edges_between(&w[0], &w[1]).first().cloned())
                    .collect::<Vec<_>>();
                let total_weight: f64 = edges.iter().map(|e| e.weight).sum();
                return Some(Lineage {
                    source_id: source_id.to_owned(),
                    target_id: target_id.to_owned(),
                    path,
                    edges,
                    total_weight,
                });
            }
            for edge in self.outgoing_edges(&current) {
                if visited.contains(&edge.to) {
                    continue;
                }
                visited.insert(edge.to.clone());
                parent.insert(edge.to.clone(), current.clone());
                queue.push_back(edge.to.clone());
            }
        }
        None
    }

    fn reconstruct_path(parent: &HashMap<String, String>, source: &str, target: &str) -> Vec<String> {
        let mut path = vec![target.to_owned()];
        let mut current = target.to_owned();
        while current != source {
            if let Some(p) = parent.get(&current) {
                path.push(p.clone());
                current = p.clone();
            } else {
                break;
            }
        }
        path.reverse();
        path
    }

    fn edges_between(&self, from: &str, to: &str) -> Vec<ProvenanceEdge> {
        self.edges
            .iter()
            .filter(|e| e.from == from && e.to == to)
            .cloned()
            .collect()
    }

    pub fn ancestors(&self, node_id: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut queue = VecDeque::new();
        for edge in self.incoming_edges(node_id) {
            queue.push_back(edge.from.clone());
        }
        while let Some(current) = queue.pop_front() {
            if seen.insert(current.clone()) {
                for edge in self.incoming_edges(&current) {
                    queue.push_back(edge.from.clone());
                }
            }
        }
        seen.into_iter().collect()
    }

    pub fn descendants(&self, node_id: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut queue = VecDeque::new();
        for edge in self.outgoing_edges(node_id) {
            queue.push_back(edge.to.clone());
        }
        while let Some(current) = queue.pop_front() {
            if seen.insert(current.clone()) {
                for edge in self.outgoing_edges(&current) {
                    queue.push_back(edge.to.clone());
                }
            }
        }
        seen.into_iter().collect()
    }

    pub fn causal_influence(&self, cause_id: &str, effect_id: &str) -> Option<CausalInfluence> {
        let lineage = self.trace_lineage(cause_id, effect_id)?;
        let confidence = 1.0 / (1.0 + lineage.path.len() as f64 * 0.15);
        let edge_kinds: Vec<ProvenanceEdgeKind> = lineage.edges.iter().map(|e| e.kind).collect();
        Some(CausalInfluence {
            cause_id: cause_id.to_owned(),
            effect_id: effect_id.to_owned(),
            confidence,
            path_length: lineage.path.len(),
            edge_kinds,
        })
    }

    pub fn roots(&self) -> Vec<String> {
        self.nodes
            .keys()
            .filter(|id| self.incoming_edges(id).is_empty())
            .cloned()
            .collect()
    }

    pub fn leaves(&self) -> Vec<String> {
        self.nodes
            .keys()
            .filter(|id| self.outgoing_edges(id).is_empty())
            .cloned()
            .collect()
    }

    pub fn all_paths(&self, source_id: &str, target_id: &str, max_depth: usize) -> Vec<Vec<String>> {
        let mut paths = Vec::new();
        let mut current_path = vec![source_id.to_owned()];
        let mut visited = HashSet::new();
        visited.insert(source_id.to_owned());
        self.all_paths_dfs(
            source_id,
            target_id,
            &mut current_path,
            &mut visited,
            &mut paths,
            max_depth,
        );
        paths
    }

    fn all_paths_dfs(
        &self,
        current: &str,
        target: &str,
        path: &mut Vec<String>,
        visited: &mut HashSet<String>,
        paths: &mut Vec<Vec<String>>,
        max_depth: usize,
    ) {
        if path.len() > max_depth {
            return;
        }
        if current == target {
            paths.push(path.clone());
            return;
        }
        for edge in self.outgoing_edges(current) {
            if !visited.contains(&edge.to) {
                visited.insert(edge.to.clone());
                path.push(edge.to.clone());
                self.all_paths_dfs(&edge.to, target, path, visited, paths, max_depth);
                path.pop();
                visited.remove(&edge.to);
            }
        }
    }

    pub fn subgraph(&self, root_id: &str, max_depth: usize) -> ProvenanceGraph {
        let mut sub = ProvenanceGraph::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((root_id.to_owned(), 0usize));
        if let Some(node) = self.nodes.get(root_id) {
            sub.add_node(node.clone());
        }
        while let Some((node_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id.clone());
            for edge in self.outgoing_edges(&node_id) {
                if let Some(n) = self.nodes.get(&edge.to) {
                    sub.add_node(n.clone());
                }
                sub.add_edge(edge.clone());
                queue.push_back((edge.to.clone(), depth + 1));
            }
        }
        sub
    }

    pub fn topological_sort(&self) -> Vec<String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(id.clone(), self.incoming_edges(id).len());
        }
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut order = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node.clone());
            for edge in self.outgoing_edges(&node) {
                if let Some(d) = in_degree.get_mut(&edge.to) {
                    *d -= 1;
                    if *d == 0 {
                        queue.push_back(edge.to.clone());
                    }
                }
            }
        }
        order
    }

    pub fn signature(&self) -> String {
        let mut buffer = String::new();
        let mut sorted_nodes: Vec<_> = self.nodes.keys().collect();
        sorted_nodes.sort();
        for id in &sorted_nodes {
            buffer.push_str(id);
            buffer.push('|');
        }
        for edge in &self.edges {
            buffer.push_str(&format!("{}->{}:{:?}", edge.from, edge.to, edge.kind));
            buffer.push(';');
        }
        blake3::hash(buffer.as_bytes()).to_hex().to_string()[..16].to_string()
    }

    pub fn to_event_records(&self, session_id: &str) -> Vec<runlens_storage::EventRecord> {
        self.edges
            .iter()
            .map(|e| runlens_storage::EventRecord {
                session_id: session_id.to_owned(),
                event_id: format!("prov-{}-{}", e.from, e.to),
                sequence: 0,
                kind: format!("provenance.{:?}", e.kind),
                payload_json: serde_json::to_string(e).unwrap_or_default(),
                timestamp_ns: e.timestamp_ns,
                hash: "".into(),
            })
            .collect()
    }
}

impl Default for ProvenanceGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: ProvenanceNodeKind, label: &str, ts: i64) -> ProvenanceNode {
        ProvenanceNode {
            id: id.into(),
            kind,
            label: label.into(),
            timestamp_ns: ts,
            metadata: HashMap::new(),
        }
    }

    fn edge(from: &str, to: &str, kind: ProvenanceEdgeKind, ts: i64, weight: f64) -> ProvenanceEdge {
        ProvenanceEdge {
            from: from.into(),
            to: to.into(),
            kind,
            timestamp_ns: ts,
            weight,
            details: String::new(),
        }
    }

    #[test]
    fn test_add_nodes_and_edges() {
        let mut g = ProvenanceGraph::new();
        g.add_node(node(
            "orders.csv",
            ProvenanceNodeKind::DataSource,
            "raw orders feed",
            100,
        ));
        g.add_node(node(
            "daily_rollup",
            ProvenanceNodeKind::Computation,
            "rollup by day",
            200,
        ));
        g.add_edge(edge(
            "orders.csv",
            "daily_rollup",
            ProvenanceEdgeKind::DerivedFrom,
            150,
            1.0,
        ));
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn test_outgoing_incoming_edges() {
        let mut g = ProvenanceGraph::new();
        for (name, kind, label, ts) in [
            ("src_a", ProvenanceNodeKind::DataSource, "feed A", 100),
            ("middle", ProvenanceNodeKind::Computation, "builder", 200),
            ("out_b", ProvenanceNodeKind::DataResult, "result B", 300),
        ] {
            g.add_node(node(name, kind, label, ts));
        }
        g.add_edge(edge("src_a", "middle", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        g.add_edge(edge("middle", "out_b", ProvenanceEdgeKind::DerivedFrom, 250, 0.8));
        assert_eq!(g.outgoing_edges("middle").len(), 1);
        assert_eq!(g.incoming_edges("middle").len(), 1);
    }

    #[test]
    fn test_trace_lineage() {
        let mut g = ProvenanceGraph::new();
        g.add_node(node("raw_logs", ProvenanceNodeKind::DataSource, "parsed logs", 100));
        g.add_node(node("cleaned", ProvenanceNodeKind::Computation, "cleaning step", 200));
        g.add_node(node("final_ds", ProvenanceNodeKind::DataResult, "final dataset", 300));
        g.add_edge(edge("raw_logs", "cleaned", ProvenanceEdgeKind::DerivedFrom, 150, 0.8));
        g.add_edge(edge("cleaned", "final_ds", ProvenanceEdgeKind::Transformed, 250, 1.1));
        let lineage = g.trace_lineage("raw_logs", "final_ds").unwrap();
        assert_eq!(lineage.path, vec!["raw_logs", "cleaned", "final_ds"]);
        assert_eq!(lineage.edges.len(), 2);
        assert!((lineage.total_weight - 1.9).abs() < 0.01);
    }

    #[test]
    fn test_trace_lineage_not_found() {
        let mut g = ProvenanceGraph::new();
        g.add_node(node("left", ProvenanceNodeKind::DataSource, "left side", 100));
        g.add_node(node("right", ProvenanceNodeKind::DataSource, "right side", 200));
        assert!(g.trace_lineage("left", "right").is_none());
    }

    #[test]
    fn test_ancestors() {
        let mut g = ProvenanceGraph::new();
        let steps = [("open", 100), ("parse", 200), ("validate", 300), ("publish", 400)];
        for (idx, (name, ts)) in steps.iter().enumerate() {
            let kind = if idx == 0 {
                ProvenanceNodeKind::DataSource
            } else {
                ProvenanceNodeKind::Computation
            };
            g.add_node(node(name, kind, name, *ts));
        }
        g.add_edge(edge("open", "parse", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        g.add_edge(edge("parse", "validate", ProvenanceEdgeKind::DerivedFrom, 250, 1.0));
        g.add_edge(edge("validate", "publish", ProvenanceEdgeKind::DerivedFrom, 350, 1.0));
        let ancestors = g.ancestors("publish");
        assert_eq!(ancestors.len(), 3);
        assert!(ancestors.contains(&"open".to_string()));
        assert!(ancestors.contains(&"parse".to_string()));
        assert!(ancestors.contains(&"validate".to_string()));
    }

    #[test]
    fn test_descendants() {
        let mut g = ProvenanceGraph::new();
        g.add_node(node("tap", ProvenanceNodeKind::DataSource, "source tap", 100));
        g.add_node(node("agg", ProvenanceNodeKind::Computation, "aggregate", 200));
        g.add_node(node("report", ProvenanceNodeKind::DataResult, "report out", 300));
        g.add_edge(edge("tap", "agg", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        g.add_edge(edge("tap", "report", ProvenanceEdgeKind::Caused, 160, 0.5));
        let desc = g.descendants("tap");
        assert_eq!(desc.len(), 2);
        assert!(desc.contains(&"agg".to_string()));
        assert!(desc.contains(&"report".to_string()));
    }

    #[test]
    fn test_causal_influence() {
        let mut g = ProvenanceGraph::new();
        g.add_node(node("deploy_click", ProvenanceNodeKind::Event, "clicked deploy", 100));
        g.add_node(node("job_start", ProvenanceNodeKind::Event, "job started", 200));
        g.add_edge(edge("deploy_click", "job_start", ProvenanceEdgeKind::Caused, 150, 1.0));
        let influence = g.causal_influence("deploy_click", "job_start").unwrap();
        assert!(influence.confidence > 0.0 && influence.confidence < 1.0);
        assert_eq!(influence.path_length, 2);
        assert_eq!(influence.edge_kinds, vec![ProvenanceEdgeKind::Caused]);
    }

    #[test]
    fn test_roots_and_leaves() {
        let mut g = ProvenanceGraph::new();
        g.add_node(node("seed", ProvenanceNodeKind::DataSource, "seed data", 100));
        g.add_node(node("step1", ProvenanceNodeKind::Computation, "first pass", 200));
        g.add_node(node("output", ProvenanceNodeKind::DataResult, "deliverable", 300));
        g.add_edge(edge("seed", "step1", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        g.add_edge(edge("step1", "output", ProvenanceEdgeKind::DerivedFrom, 250, 1.0));
        let roots = g.roots();
        assert_eq!(roots, vec!["seed"]);
        let leaves = g.leaves();
        assert_eq!(leaves, vec!["output"]);
    }

    #[test]
    fn test_all_paths() {
        let mut g = ProvenanceGraph::new();
        g.add_node(node("start", ProvenanceNodeKind::DataSource, "entry", 100));
        g.add_node(node("via_left", ProvenanceNodeKind::Computation, "left branch", 200));
        g.add_node(node("via_right", ProvenanceNodeKind::Computation, "right branch", 300));
        g.add_node(node("finish", ProvenanceNodeKind::DataResult, "exit", 400));
        g.add_edge(edge("start", "via_left", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        g.add_edge(edge("start", "via_right", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        g.add_edge(edge("via_left", "finish", ProvenanceEdgeKind::DerivedFrom, 250, 1.0));
        g.add_edge(edge("via_right", "finish", ProvenanceEdgeKind::DerivedFrom, 350, 1.0));
        let paths = g.all_paths("start", "finish", 10);
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_topological_sort() {
        let mut g = ProvenanceGraph::new();
        g.add_node(node("ingest", ProvenanceNodeKind::DataSource, "ingest", 100));
        g.add_node(node("transform", ProvenanceNodeKind::Computation, "transform", 200));
        g.add_node(node("load", ProvenanceNodeKind::DataResult, "load", 300));
        g.add_edge(edge("ingest", "transform", ProvenanceEdgeKind::DerivedFrom, 100, 1.0));
        g.add_edge(edge("transform", "load", ProvenanceEdgeKind::DerivedFrom, 200, 1.0));
        let topo = g.topological_sort();
        assert_eq!(topo.len(), 3);
        let ingest = topo.iter().position(|s| s == "ingest").unwrap();
        let transform = topo.iter().position(|s| s == "transform").unwrap();
        let load = topo.iter().position(|s| s == "load").unwrap();
        assert!(ingest < transform);
        assert!(transform < load);
    }

    #[test]
    fn test_subgraph() {
        let mut g = ProvenanceGraph::new();
        for i in 0..5 {
            g.add_node(node(
                &format!("n{}", i),
                ProvenanceNodeKind::Computation,
                &format!("node {}", i),
                i as i64 * 100,
            ));
        }
        g.add_edge(edge("n0", "n1", ProvenanceEdgeKind::DerivedFrom, 50, 1.0));
        g.add_edge(edge("n1", "n2", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        g.add_edge(edge("n2", "n3", ProvenanceEdgeKind::DerivedFrom, 250, 1.0));
        g.add_edge(edge("n3", "n4", ProvenanceEdgeKind::DerivedFrom, 350, 1.0));
        let sub = g.subgraph("n1", 2);
        assert!(sub.node_count() >= 3);
        assert!(sub.node_count() <= 4);
    }

    #[test]
    fn test_signature() {
        let mut g1 = ProvenanceGraph::new();
        g1.add_node(node("a", ProvenanceNodeKind::DataSource, "A", 100));
        g1.add_node(node("b", ProvenanceNodeKind::Computation, "B", 200));
        g1.add_edge(edge("a", "b", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        let mut g2 = ProvenanceGraph::new();
        g2.add_node(node("a", ProvenanceNodeKind::DataSource, "A", 100));
        g2.add_node(node("b", ProvenanceNodeKind::Computation, "B", 200));
        g2.add_edge(edge("a", "b", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        assert_eq!(g1.signature(), g2.signature());
        g2.add_edge(edge("b", "a", ProvenanceEdgeKind::Caused, 160, 1.0));
        assert_ne!(g1.signature(), g2.signature());
    }

    #[test]
    fn test_to_event_records() {
        let mut g = ProvenanceGraph::new();
        g.add_node(node("in", ProvenanceNodeKind::DataSource, "in", 100));
        g.add_node(node("out", ProvenanceNodeKind::Computation, "out", 200));
        g.add_edge(edge("in", "out", ProvenanceEdgeKind::DerivedFrom, 150, 1.0));
        let records = g.to_event_records("sess1");
        assert_eq!(records.len(), 1);
        assert!(records[0].kind.starts_with("provenance."));
    }
}
