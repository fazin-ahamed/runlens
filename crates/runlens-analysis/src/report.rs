use crate::{SessionAnalysis, Verdict};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunContext {
    pub project_id: String,
    pub baseline_session_id: Option<String>,
    pub candidate_session_id: String,
    pub started_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
}

impl RunContext {
    pub fn new(candidate_session_id: impl Into<String>) -> Self {
        Self {
            project_id: String::new(),
            baseline_session_id: None,
            candidate_session_id: candidate_session_id.into(),
            started_at: None,
            description: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BugReport {
    pub title: String,
    pub verdict: Verdict,
    pub summary: String,
    pub evidence: Vec<EvidenceItem>,
    pub markdown: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidenceItem {
    pub event_kind: String,
    pub event_sequence: Option<u64>,
    pub note: String,
}

pub fn generate_bug_report(analysis: &SessionAnalysis, context: &RunContext) -> Option<BugReport> {
    if analysis.verdict == Verdict::Clean {
        return None;
    }
    let mut title = format!("RunLens analysis: {}", analysis.verdict.as_str());
    if let Some(desc) = context.description.as_deref() {
        if !desc.is_empty() {
            title = format!("{desc}: {}", analysis.verdict.as_str());
        }
    }
    let broken = analysis.verdict == Verdict::Broken;
    let summary = if broken {
        format!(
            "The candidate session diverged from the baseline with {} divergence(s), including at least one new failure kind. {}",
            analysis.comparison.divergences.len(),
            lead_divergence(analysis)
        )
    } else {
        format!(
            "The candidate session diverged from the baseline with {} divergence(s). {}",
            analysis.comparison.divergences.len(),
            lead_divergence(analysis)
        )
    };
    let mut evidence: Vec<EvidenceItem> = Vec::new();
    for d in analysis.top_divergences.iter().take(5) {
        evidence.push(EvidenceItem {
            event_kind: d.title.clone(),
            event_sequence: d.evidence_event_sequence,
            note: d.summary.clone(),
        });
    }
    let markdown = render_markdown(analysis, context, &title, &summary);
    Some(BugReport {
        title,
        verdict: analysis.verdict,
        summary,
        evidence,
        markdown,
    })
}

fn lead_divergence(analysis: &SessionAnalysis) -> String {
    match analysis.top_divergences.first() {
        Some(d) => format!(
            "The most significant finding is '{}' (score {}).",
            d.title,
            d.total_score()
        ),
        None => "No significant findings were detected.".to_string(),
    }
}

fn render_markdown(analysis: &SessionAnalysis, context: &RunContext, title: &str, summary: &str) -> String {
    let mut out = String::new();
    out.push_str("# ");
    out.push_str(title);
    out.push('\n');
    out.push('\n');
    out.push_str("**Verdict:** `");
    out.push_str(analysis.verdict.as_str());
    out.push_str("`\n\n");
    if let Some(desc) = context.description.as_deref() {
        if !desc.is_empty() {
            out.push_str("**Description:** ");
            out.push_str(desc);
            out.push_str("\n\n");
        }
    }
    out.push_str("## Summary\n\n");
    out.push_str(summary);
    out.push_str("\n\n");
    out.push_str("## Run context\n\n");
    out.push_str(&format!(
        "- **Project:** `{}`\n",
        if context.project_id.is_empty() {
            "unknown"
        } else {
            context.project_id.as_str()
        }
    ));
    match &context.baseline_session_id {
        Some(id) => out.push_str(&format!("- **Baseline session:** `{id}`\n")),
        None => out.push_str("- **Baseline session:** none\n"),
    }
    out.push_str(&format!(
        "- **Candidate session:** `{}`\n",
        context.candidate_session_id
    ));
    match context.started_at {
        Some(ts) => out.push_str(&format!("- **Started at:** {ts}\n")),
        None => out.push_str("- **Started at:** unknown\n"),
    }
    out.push_str(&format!(
        "- **Event counts:** baseline {}, candidate {}\n",
        analysis.comparison.baseline_event_count, analysis.comparison.candidate_event_count
    ));
    out.push_str("\n## Divergences\n\n");
    if analysis.top_divergences.is_empty() {
        out.push_str("_No divergences ranked above the cutoff._\n");
    } else {
        for (i, d) in analysis.top_divergences.iter().enumerate() {
            out.push_str(&format!(
                "{}. **{}** (score {}, severity `{}`)\n",
                i + 1,
                d.title,
                d.total_score(),
                d.severity.as_str()
            ));
            out.push_str(&format!("   {}\n", d.summary));
            for f in &d.factors {
                out.push_str(&format!("   - {} (weight {})\n", f.reason, f.weight));
            }
            if let Some(seq) = d.evidence_event_sequence {
                out.push_str(&format!("   - Evidence: event sequence `{seq}`\n"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::RunContext;
    use super::*;
    use crate::analyze_sessions;
    use chrono::{TimeZone, Utc};
    use runlens_core::model::{EventSource, PrivacyClassification, Severity};

    fn ev(seq: u64, kind: &str, severity: Severity) -> runlens_core::model::Event {
        let ts = Utc.timestamp_opt(0, 0).single().unwrap();
        runlens_core::model::Event {
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
    fn clean_analysis_yields_no_report() {
        let base = vec![ev(0, "session.started", Severity::Info)];
        let analysis = analyze_sessions(&base, &base);
        let ctx = RunContext::new("01H00000000000000000000003");
        assert!(generate_bug_report(&analysis, &ctx).is_none());
    }

    #[test]
    fn broken_analysis_yields_markdown_report() {
        let base = vec![ev(0, "session.started", Severity::Info)];
        let cand = vec![
            ev(0, "session.started", Severity::Info),
            ev(1, "process.exited", Severity::Error),
        ];
        let analysis = analyze_sessions(&base, &cand);
        let mut ctx = RunContext::new("01H00000000000000000000003");
        ctx.description = Some("integration probe".into());
        let report = generate_bug_report(&analysis, &ctx).expect("report exists");
        assert_eq!(report.verdict, Verdict::Broken);
        assert!(report.title.contains("broken"));
        assert!(report.markdown.contains("integration probe"));
        assert!(report.markdown.contains("Divergences"));
        assert!(report.markdown.contains("New failure kind"));
        assert!(!report.evidence.is_empty());
    }

    #[test]
    fn report_evidence_carries_sequence() {
        let base = vec![ev(0, "session.started", Severity::Info)];
        let cand = vec![
            ev(0, "session.started", Severity::Info),
            ev(1, "file.modified", Severity::Info),
            ev(2, "process.exited", Severity::Fatal),
        ];
        let analysis = analyze_sessions(&base, &cand);
        let ctx = RunContext::new("01H00000000000000000000003");
        let report = generate_bug_report(&analysis, &ctx).expect("report exists");
        let seqs: Vec<Option<u64>> = report.evidence.iter().map(|e| e.event_sequence).collect();
        assert!(seqs.iter().any(|s| s.is_some()));
    }
}
