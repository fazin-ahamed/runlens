use clap::Parser;

#[derive(Debug, Parser)]
pub struct GraphArgs {
    #[command(subcommand)]
    pub action: GraphAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum GraphAction {
    Trace {
        trace_id: String,
        #[arg(long)]
        json: bool,
    },
    Critical {
        trace_id: String,
        #[arg(long)]
        json: bool,
    },
    Compare {
        session_a: String,
        session_b: String,
        #[arg(long)]
        json: bool,
    },
    Chain {
        trace_id: String,
        span_id: String,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(args: &GraphArgs, workspace: &crate::paths::WorkspacePaths) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let builder = runlens_graph::graph::GraphBuilder::new(&repo);

    match &args.action {
        GraphAction::Trace { trace_id, json } => {
            let graph = builder.load(trace_id)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&graph)?);
            } else {
                println!(
                    "Trace: {} ({} nodes, {} edges)",
                    trace_id,
                    graph.nodes.len(),
                    graph.edges.len()
                );
                for node in &graph.nodes {
                    let dur = node.duration_ms.map(|d| format!("{:.0}ms", d)).unwrap_or_default();
                    println!(
                        "  {} [{}] {:20} {:>10}",
                        node.id,
                        node.name,
                        node.source.as_deref().unwrap_or("?"),
                        dur
                    );
                }
            }
        },
        GraphAction::Critical { trace_id, json } => {
            let graph = builder.load(trace_id)?;
            let path = runlens_graph::critical::critical_path(&graph);
            if *json {
                println!("{}", serde_json::to_string_pretty(&path)?);
            } else {
                let total_dur: f64 = path.iter().filter_map(|n| n.duration_ms).sum();
                println!("Critical path ({} spans, {:.0}ms total):", path.len(), total_dur);
                for node in &path {
                    let dur = node.duration_ms.map(|d| format!("{:.0}ms", d)).unwrap_or_default();
                    println!("  {} [{}] {:>10}", node.id, node.name, dur);
                }
            }
        },
        GraphAction::Compare {
            session_a,
            session_b,
            json,
        } => {
            let graph_a = builder.load_session(session_a)?;
            let graph_b = builder.load_session(session_b)?;
            let diff = runlens_graph::diff::compare(&graph_a, &graph_b);
            if *json {
                println!("{}", serde_json::to_string_pretty(&diff)?);
            } else {
                println!("Graph comparison:");
                println!(
                    "  Added: {} nodes, {} edges",
                    diff.added_nodes.len(),
                    diff.added_edges.len()
                );
                println!(
                    "  Removed: {} nodes, {} edges",
                    diff.removed_nodes.len(),
                    diff.removed_edges.len()
                );
                println!("  Changed: {} spans", diff.changed_spans.len());
                for cs in &diff.changed_spans {
                    println!(
                        "    {}: {:.0?}ms -> {:.0?}ms",
                        cs.name, cs.old_duration_ms, cs.new_duration_ms
                    );
                }
            }
        },
        GraphAction::Chain {
            trace_id,
            span_id,
            json,
        } => {
            let graph = builder.load(trace_id)?;
            let chain = runlens_graph::span::follow_chain(&graph, span_id);
            if *json {
                println!("{}", serde_json::to_string_pretty(&chain)?);
            } else {
                println!("Span chain ({} spans):", chain.len());
                for node in &chain {
                    println!("  {} [{}]", node.id, node.name);
                }
            }
        },
    }
    Ok(())
}
