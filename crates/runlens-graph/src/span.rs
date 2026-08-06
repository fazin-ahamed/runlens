use crate::graph::{EventGraph, GraphNode};

pub fn follow_chain(graph: &EventGraph, span_id: &str) -> Vec<GraphNode> {
    let mut chain = Vec::new();
    let mut current = span_id.to_string();

    while let Some(node) = graph.nodes.iter().find(|n| n.id == current) {
        chain.push(node.clone());

        let Some(next) = graph.edges.iter().find(|e| e.from == current).map(|e| e.to.clone()) else {
            break;
        };
        current = next;
    }

    chain
}
