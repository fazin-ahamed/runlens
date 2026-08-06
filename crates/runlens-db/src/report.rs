use crate::detector::AnalysisResult;

pub fn to_json(result: &AnalysisResult) -> String {
    serde_json::to_string_pretty(result).unwrap()
}

pub fn to_text(result: &AnalysisResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("Session: {}\n\n", result.session_id));

    out.push_str("=== N+1 Query Groups ===\n");
    if result.n_plus_one.is_empty() {
        out.push_str("  (none detected)\n");
    } else {
        for g in &result.n_plus_one {
            out.push_str(&format!("\n  SQL: {}\n", g.normalized_sql));
            out.push_str(&format!(
                "  Count: {} | Total: {}ms\n",
                g.count,
                g.total_duration_ns / 1_000_000
            ));
            out.push_str(&format!("  Example: {}\n", g.example_sql));
        }
    }

    out.push_str("\n=== Slow Queries ===\n");
    if result.slow_queries.is_empty() {
        out.push_str("  (none detected)\n");
    } else {
        for q in &result.slow_queries {
            out.push_str(&format!("  {}ms | {}\n", q.duration_ns / 1_000_000, q.sql));
            out.push_str(&format!("  Event: {}\n", q.event_id));
        }
    }
    out
}
