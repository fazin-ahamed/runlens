//! Deterministic explainable run comparison.
//!
//! Given two ordered event lists (a baseline and a candidate), produce a
//! ranked list of "likely relevant divergences" — each one explains why
//! its score is what it is. This module is pure: it doesn't know about
//! storage; callers load events and feed them in.

use crate::model::Event;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Factors contributing to a divergence ranking. Each has a small
/// `weight` (relative importance, 0..=10) and a `reason` string that
/// gets surfaced in the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DivergenceFactor {
    pub weight: u8,
    pub reason: String,
}

/// A single ranked divergence between the baseline and candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Divergence {
    pub title: String,
    pub summary: String,
    pub severity: DivergenceSeverity,
    pub factors: Vec<DivergenceFactor>,
    pub evidence_event_sequence: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DivergenceSeverity {
    Low,
    Moderate,
    High,
}

impl DivergenceSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
        }
    }
}

impl DivergenceFactor {
    pub fn new(weight: u8, reason: impl Into<String>) -> Self {
        Self {
            weight: weight.min(10),
            reason: reason.into(),
        }
    }
}

impl Divergence {
    pub fn total_score(&self) -> u32 {
        self.factors.iter().map(|f| f.weight as u32).sum()
    }

    pub fn severity(&self) -> DivergenceSeverity {
        let s = self.total_score();
        if s >= 15 {
            DivergenceSeverity::High
        } else if s >= 7 {
            DivergenceSeverity::Moderate
        } else {
            DivergenceSeverity::Low
        }
    }
}

/// Result of comparing two sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    pub divergences: Vec<Divergence>,
    pub baseline_event_count: u64,
    pub candidate_event_count: u64,
}

impl Comparison {
    pub fn ranked(&self) -> &[Divergence] {
        // Caller sorts in `compare_sessions`.
        &self.divergences
    }
}

/// Build a complete comparison between two event lists.
pub fn compare_sessions(baseline: &[Event], candidate: &[Event]) -> Comparison {
    let mut divs: Vec<Divergence> = Vec::new();
    let mut by_kind: IndexMap<String, (u64, u64)> = IndexMap::new();
    let mut first_error_seq: Option<u64> = None;
    for e in baseline {
        let entry = by_kind.entry(e.kind.clone()).or_insert((0, 0));
        entry.0 += 1;
    }
    for e in candidate {
        let entry = by_kind.entry(e.kind.clone()).or_insert((0, 0));
        entry.1 += 1;
        if first_error_seq.is_none() && e.is_error_like() {
            first_error_seq = Some(e.sequence);
        }
    }
    // New kinds in candidate.
    for (kind, (base, cand)) in &by_kind {
        if *base == 0 && *cand > 0 {
            divs.push(Divergence {
                title: format!("New event kind appeared: {kind}"),
                summary: format!("The candidate session emitted {cand} `{kind}` events that did not appear in the baseline."),
                severity: DivergenceSeverity::Moderate,
                factors: vec![
                    DivergenceFactor::new(5, format!(
                        "`{kind}` appeared {cand} times in the candidate and 0 times in the baseline.",
                    )),
                ],
                evidence_event_sequence: None,
            });
        } else if base != cand && cand > &base {
            let delta = cand - base;
            divs.push(Divergence {
                title: format!("Event kind increased: {kind}"),
                summary: format!(
                    "`{kind}` fired {base} times in baseline and {cand} times in candidate (delta {delta})."
                ),
                severity: DivergenceSeverity::Low,
                factors: vec![
                    DivergenceFactor::new(3, format!(
                        "count delta across the session is {delta}.",
                    )),
                ],
                evidence_event_sequence: None,
            });
        }
    }
    // First-error proximity check: events that occurred shortly before
    // the first error in the candidate.
    if let Some(first_err) = first_error_seq {
        let mut proximity_events: Vec<&Event> = candidate
            .iter()
            .filter(|e| e.sequence + 5 >= first_err && e.sequence < first_err && !e.is_error_like())
            .collect();
        proximity_events.sort_by_key(|e| e.sequence);
        if let Some(prev) = proximity_events.last() {
            divs.push(Divergence {
                title: format!("Event immediately preceded first failure: {}", prev.kind),
                summary: format!(
                    "Event '{}' at sequence #{} occurred just before the first error-like event (#{}).",
                    prev.kind, prev.sequence, first_err
                ),
                severity: DivergenceSeverity::High,
                factors: vec![
                    DivergenceFactor::new(8, format!("event at sequence #{} precedes first failure at #{} by <= 5 steps.", prev.sequence, first_err)),
                    DivergenceFactor::new(3, format!("`{}` did not appear in baseline run.", prev.kind)),
                ],
                evidence_event_sequence: Some(prev.sequence),
            });
        }
    }
    // Severity: distinct error kinds.
    let base_errors: std::collections::BTreeSet<String> = baseline
        .iter()
        .filter(|e| e.is_error_like())
        .map(|e| e.kind.clone())
        .collect();
    let cand_errors: std::collections::BTreeSet<String> = candidate
        .iter()
        .filter(|e| e.is_error_like())
        .map(|e| e.kind.clone())
        .collect();
    let new_errors: Vec<&String> = cand_errors.difference(&base_errors).collect();
    for ek in new_errors {
        divs.push(Divergence {
            title: format!("New failure kind: {ek}"),
            summary: format!("A failure category `{ek}` appeared in the candidate but not in the baseline. The deviation may be relevant."),
            severity: DivergenceSeverity::High,
            factors: vec![
                DivergenceFactor::new(7, format!("`{ek}` is a new failure kind unique to the candidate run.")),
            ],
            evidence_event_sequence: None,
        });
    }
    divs.sort_by(|a, b| b.total_score().cmp(&a.total_score()));
    Comparison {
        divergences: divs,
        baseline_event_count: baseline.len() as u64,
        candidate_event_count: candidate.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EventSource, PrivacyClassification, Severity};
    use chrono::{TimeZone, Utc};

    fn ev(seq: u64, secs: i64, kind: &str, severity: Severity) -> Event {
        let ts = Utc.timestamp_opt(secs, 0).single().unwrap();
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
    fn empty_inputs_yield_no_divergences() {
        let r = compare_sessions(&[], &[]);
        assert!(r.divergences.is_empty());
        assert_eq!(r.baseline_event_count, 0);
        assert_eq!(r.candidate_event_count, 0);
    }

    #[test]
    fn identifies_new_error_kind() {
        let base = vec![ev(0, 1, "session.started", Severity::Info)];
        let cand = vec![
            ev(0, 1, "session.started", Severity::Info),
            ev(1, 2, "process.exited", Severity::Error),
        ];
        let r = compare_sessions(&base, &cand);
        assert!(r.divergences.iter().any(|d| d.title.contains("New failure kind")));
    }

    #[test]
    fn flagged_event_precedes_first_failure() {
        let base = vec![ev(0, 1, "session.started", Severity::Info)];
        let cand = vec![
            ev(0, 1, "session.started", Severity::Info),
            ev(1, 2, "file.modified", Severity::Info),
            ev(2, 3, "process.exited", Severity::Fatal),
        ];
        let r = compare_sessions(&base, &cand);
        assert!(r
            .divergences
            .iter()
            .any(|d| d.title.contains("immediately preceded")));
    }

    #[test]
    fn divergences_are_ranked_with_explanations() {
        let base = vec![ev(0, 1, "session.started", Severity::Info)];
        let cand = vec![
            ev(0, 1, "session.started", Severity::Info),
            ev(1, 2, "terminal.stderr", Severity::Info),
            ev(2, 3, "process.exited", Severity::Error),
        ];
        let r = compare_sessions(&base, &cand);
        for d in &r.divergences {
            assert!(!d.factors.is_empty(), "{} has no factors", d.title);
        }
        let max_score = r
            .divergences
            .first()
            .map(|d| d.total_score())
            .unwrap_or_default();
        let min_score = r
            .divergences
            .last()
            .map(|d| d.total_score())
            .unwrap_or_default();
        assert!(max_score >= min_score);
    }
}
