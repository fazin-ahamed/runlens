pub mod report;

use runlens_core::compare::{compare_sessions, Comparison, Divergence};
use runlens_core::model::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Clean,
    Suspicious,
    Broken,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Suspicious => "suspicious",
            Self::Broken => "broken",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionAnalysis {
    pub verdict: Verdict,
    pub comparison: Comparison,
    pub top_divergences: Vec<Divergence>,
}

pub fn analyze_sessions(baseline: &[Event], candidate: &[Event]) -> SessionAnalysis {
    let comparison = compare_sessions(baseline, candidate);
    let has_new_failure = comparison
        .divergences
        .iter()
        .any(|d| d.title.starts_with("New failure kind"));
    let verdict = if comparison.divergences.is_empty() {
        Verdict::Clean
    } else if has_new_failure {
        Verdict::Broken
    } else {
        Verdict::Suspicious
    };
    let top_divergences = comparison.divergences.iter().take(10).cloned().collect();
    SessionAnalysis {
        verdict,
        comparison,
        top_divergences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use runlens_core::model::{EventSource, PrivacyClassification, Severity};

    fn ev(seq: u64, kind: &str, severity: Severity) -> Event {
        let ts = Utc.timestamp_opt(0, 0).single().unwrap();
        Event {
            event_id: format!("01H{seq:025}"),
            session_id: "01H00000000000000000000001".into(),
            project_id: "01H00000000000000000000002".into(),
            sequence: seq,
            source: EventSource::Core,
            kind: kind.into(),
            severity,
            utc_timestamp: ts,
            monotonic_ns: 0,
            duration_ns: None,
            correlation_id: None,
            parent_event_id: None,
            payload_version: 1,
            payload: serde_json::json!({}),
            classification: PrivacyClassification::Internal,
            previous_hash: None,
            current_hash: None,
        }
    }

    #[test]
    fn identical_sessions_are_clean() {
        let base = vec![ev(0, "session.started", Severity::Info)];
        let cand = base.clone();
        let a = analyze_sessions(&base, &cand);
        assert_eq!(a.verdict, Verdict::Clean);
        assert!(a.top_divergences.is_empty());
    }

    #[test]
    fn new_failure_kind_verdict_is_broken() {
        let base = vec![ev(0, "session.started", Severity::Info)];
        let cand = vec![
            ev(0, "session.started", Severity::Info),
            ev(1, "process.exited", Severity::Error),
        ];
        let a = analyze_sessions(&base, &cand);
        assert_eq!(a.verdict, Verdict::Broken);
        assert!(a.top_divergences.iter().any(|d| d.title.contains("New failure kind")));
    }

    #[test]
    fn count_drift_is_suspicious_not_broken() {
        let base = vec![ev(0, "session.started", Severity::Info); 3];
        let cand = vec![ev(0, "session.started", Severity::Info); 5];
        let a = analyze_sessions(&base, &cand);
        assert_eq!(a.verdict, Verdict::Suspicious);
    }

    #[test]
    fn top_divergences_are_ranked_by_score() {
        let base = vec![ev(0, "session.started", Severity::Info)];
        let cand = vec![
            ev(0, "session.started", Severity::Info),
            ev(1, "file.modified", Severity::Info),
            ev(2, "process.exited", Severity::Fatal),
        ];
        let a = analyze_sessions(&base, &cand);
        let scores: Vec<u32> = a.top_divergences.iter().map(|d| d.total_score()).collect();
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(scores, sorted);
        assert!(!scores.is_empty());
    }
}
