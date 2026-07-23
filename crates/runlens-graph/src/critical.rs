use crate::graph::{EventGraph, GraphNode};

pub fn critical_path(graph: &EventGraph) -> Vec<GraphNode> {
    // Rough epsilon; durations come from f64 division so exact equality is unreliable.
    let max_duration = graph
        .nodes
        .iter()
        .filter_map(|n| n.duration_ms)
        .fold(0.0_f64, f64::max);

    graph
        .nodes
        .iter()
        .filter(|n| n.duration_ms.map_or(false, |d| (d - max_duration).abs() < 0.001))
        .cloned()
        .collect()
}
