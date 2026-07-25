use std::collections::HashMap;

use runlens_core::event_v2::EventV2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPlusOneGroup {
    pub normalized_sql: String,
    pub count: usize,
    pub total_duration_ns: i64,
    pub example_sql: String,
    pub timestamps_sec: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQuery {
    pub sql: String,
    pub duration_ns: i64,
    pub timestamp: String,
    pub event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub session_id: String,
    pub n_plus_one: Vec<NPlusOneGroup>,
    pub slow_queries: Vec<SlowQuery>,
}

pub fn detect_n_plus_one(events: &[EventV2], threshold: usize) -> Vec<NPlusOneGroup> {
    let db_events: Vec<_> = events.iter().filter(|e| e.kind == "db.query").collect();

    let mut groups: HashMap<String, Vec<&EventV2>> = HashMap::new();
    for ev in &db_events {
        let key = ev
            .payload
            .get("sql_normalized")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        groups.entry(key).or_default().push(ev);
    }

    let mut detected_groups = Vec::new();
    for (sql, evs) in groups {
        if evs.len() >= threshold {
            let total_dur: i64 = evs.iter().filter_map(|e| e.duration_ns).sum();
            let example = evs[0]
                .payload
                .get("sql")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let timestamps: Vec<f64> = evs
                .iter()
                .map(|e| e.utc_timestamp.timestamp() as f64)
                .collect();
            detected_groups.push(NPlusOneGroup {
                normalized_sql: sql,
                count: evs.len(),
                total_duration_ns: total_dur,
                example_sql: example,
                timestamps_sec: timestamps,
            });
        }
    }
    detected_groups.sort_by(|a, b| b.count.cmp(&a.count));
    detected_groups
}

pub fn detect_slow_queries(events: &[EventV2], max_duration_ns: i64) -> Vec<SlowQuery> {
    let mut slow_queries: Vec<SlowQuery> = events
        .iter()
        .filter(|e| e.kind == "db.query")
        .filter(|e| e.duration_ns.map(|d| d > max_duration_ns).unwrap_or(false))
        .map(|e| SlowQuery {
            sql: e.payload
                .get("sql")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            duration_ns: e.duration_ns.unwrap_or(0),
            timestamp: e.utc_timestamp.to_rfc3339(),
            event_id: e.event_id.clone(),
        })
        .collect();
    slow_queries.sort_by(|a, b| b.duration_ns.cmp(&a.duration_ns));
    slow_queries
}

#[cfg(test)]
mod tests {
    use super::*;
    use runlens_core::event_v2::EventV2;
    use runlens_core::model::{EventSource, PrivacyClassification, Severity};
    use runlens_core::identifier::Identifier;

    fn make_query_event(sql: &str, normalized: &str) -> EventV2 {
        EventV2::new(
            Identifier::now(),
            Identifier::now(),
            Identifier::now(),
            1,
            EventSource::Sdk,
            "db.query",
            Severity::Info,
            serde_json::json!({"sql": sql, "sql_normalized": normalized}),
            PrivacyClassification::Public,
        )
    }

    fn make_query_event_with_duration(sql: &str, normalized: &str, dur_ns: i64) -> EventV2 {
        let mut ev = make_query_event(sql, normalized);
        ev.duration_ns = Some(dur_ns);
        ev
    }

    #[test]
    fn test_n_plus_one_detection() {
        let mut events = Vec::new();
        let sql = "SELECT * FROM users WHERE id = 1";
        let norm = "SELECT * FROM users WHERE id = ?";
        for _ in 0..5 {
            events.push(make_query_event_with_duration(sql, norm, 100_000));
        }
        let groups = detect_n_plus_one(&events, 3);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 5);
        assert_eq!(groups[0].normalized_sql, norm);
    }

    #[test]
    fn test_n_plus_one_below_threshold() {
        let mut events = Vec::new();
        let sql = "SELECT * FROM users WHERE id = 1";
        let norm = "SELECT * FROM users WHERE id = ?";
        for _ in 0..2 {
            events.push(make_query_event(sql, norm));
        }
        let groups = detect_n_plus_one(&events, 3);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_slow_query_detection() {
        let mut events = Vec::new();
        events.push(make_query_event_with_duration("SELECT 1", "SELECT ?", 50_000_000));
        events.push(make_query_event_with_duration("SELECT 2", "SELECT ?", 200_000_000));
        let slow = detect_slow_queries(&events, 100_000_000);
        assert_eq!(slow.len(), 1);
        assert_eq!(slow[0].sql, "SELECT 2");
    }

    #[test]
    fn test_ignores_non_db_events() {
        let mut ev = make_query_event("SELECT 1", "SELECT ?");
        ev.duration_ns = Some(100_000);
        ev.kind = "http.request".into();
        let groups = detect_n_plus_one(&[ev], 3);
        assert!(groups.is_empty());
    }
}
