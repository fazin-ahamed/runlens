#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub event_id: String,
    pub session_id: String,
    pub sequence: u64,
    pub kind: String,
    pub source: String,
    pub severity: String,
    pub summary: String,
    pub payload_json: serde_json::Value,
    pub timestamp_ns: u64,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DataCategory {
    Personal,
    Health,
    Financial,
    Behavioral,
    Technical,
    Other,
}

impl DataCategory {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Health => "health",
            Self::Financial => "financial",
            Self::Behavioral => "behavioral",
            Self::Technical => "technical",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for DataCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConsentStatus {
    Granted,
    Withdrawn,
    Expired,
    NotRequested,
}

impl ConsentStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Withdrawn => "withdrawn",
            Self::Expired => "expired",
            Self::NotRequested => "not_requested",
        }
    }
}

impl std::fmt::Display for ConsentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DataRequestKind {
    Access,
    Deletion,
    Portability,
    Rectification,
}

impl DataRequestKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Deletion => "deletion",
            Self::Portability => "portability",
            Self::Rectification => "rectification",
        }
    }
}

impl std::fmt::Display for DataRequestKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DataRequestStatus {
    Open,
    InProgress,
    Fulfilled,
    Denied,
    Expired,
}

impl DataRequestStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Fulfilled => "fulfilled",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }
}

impl std::fmt::Display for DataRequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataSubject {
    pub id: String,
    pub name: String,
    pub region: String,
    pub created_at_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataElement {
    pub id: String,
    pub name: String,
    pub category: DataCategory,
    pub retention_days: u64,
    pub owner: String,
    pub registered_at_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsentGrant {
    pub id: String,
    pub subject_id: String,
    pub purpose: String,
    pub category: DataCategory,
    pub granted_at_ns: u64,
    pub expires_at_ns: Option<u64>,
    pub status: ConsentStatus,
}

impl ConsentGrant {
    pub fn is_active(&self, at_ns: u64) -> bool {
        if self.status != ConsentStatus::Granted {
            return false;
        }
        match self.expires_at_ns {
            Some(exp) => at_ns <= exp,
            None => true,
        }
    }

    pub fn is_expired(&self, at_ns: u64) -> bool {
        if self.status == ConsentStatus::Withdrawn {
            return false;
        }
        match self.expires_at_ns {
            Some(exp) => at_ns > exp,
            None => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetentionRule {
    pub id: String,
    pub category: DataCategory,
    pub max_retention_days: u64,
    pub enforced_at_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataRequest {
    pub id: String,
    pub subject_id: String,
    pub kind: DataRequestKind,
    pub status: DataRequestStatus,
    pub requested_at_ns: u64,
    pub fulfilled_at_ns: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsentIssue {
    pub subject_id: String,
    pub purpose: String,
    pub category: DataCategory,
    pub status: ConsentStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetentionViolation {
    pub element_id: String,
    pub category: DataCategory,
    pub retention_days: u64,
    pub allowed_days: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacyFinding {
    pub severity: PrivacySeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PrivacySeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl PrivacySeverity {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

impl std::fmt::Display for PrivacySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacyReport {
    pub policy_version: String,
    pub subject_count: usize,
    pub element_count: usize,
    pub consent_count: usize,
    pub active_consent_count: usize,
    pub expired_consent_count: usize,
    pub withdrawn_consent_count: usize,
    pub open_requests: usize,
    pub consent_coverage: f64,
    pub consent_issues: Vec<ConsentIssue>,
    pub retention_violations: Vec<RetentionViolation>,
    pub findings: Vec<PrivacyFinding>,
    pub generated_at_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataInventory {
    pub subjects: Vec<DataSubject>,
    pub elements: Vec<DataElement>,
    pub consents: Vec<ConsentGrant>,
}

#[derive(Debug, Error)]
pub enum PrivacyError {
    #[error("subject not found: {0}")]
    SubjectNotFound(String),
    #[error("element not found: {0}")]
    ElementNotFound(String),
    #[error("consent not found: {0}")]
    ConsentNotFound(String),
    #[error("request not found: {0}")]
    RequestNotFound(String),
}

pub struct PrivacyManager {
    pub policy_version: String,
    subjects: BTreeMap<String, DataSubject>,
    elements: BTreeMap<String, DataElement>,
    consents: BTreeMap<String, ConsentGrant>,
    rules: Vec<RetentionRule>,
    requests: BTreeMap<String, DataRequest>,
    subject_seq: u64,
    element_seq: u64,
    consent_seq: u64,
    rule_seq: u64,
    request_seq: u64,
}

impl PrivacyManager {
    pub fn new(policy_version: &str) -> Self {
        Self {
            policy_version: policy_version.to_owned(),
            subjects: BTreeMap::new(),
            elements: BTreeMap::new(),
            consents: BTreeMap::new(),
            rules: Vec::new(),
            requests: BTreeMap::new(),
            subject_seq: 0,
            element_seq: 0,
            consent_seq: 0,
            rule_seq: 0,
            request_seq: 0,
        }
    }

    pub fn add_subject(&mut self, name: &str, region: &str, created_at_ns: u64) -> String {
        self.subject_seq += 1;
        let id = format!("sub-{:03}", self.subject_seq);
        self.subjects.insert(
            id.clone(),
            DataSubject {
                id: id.clone(),
                name: name.to_owned(),
                region: region.to_owned(),
                created_at_ns,
            },
        );
        id
    }

    pub fn register_element(
        &mut self,
        name: &str,
        category: DataCategory,
        retention_days: u64,
        owner: &str,
        registered_at_ns: u64,
    ) -> String {
        self.element_seq += 1;
        let id = format!("elem-{:03}", self.element_seq);
        self.elements.insert(
            id.clone(),
            DataElement {
                id: id.clone(),
                name: name.to_owned(),
                category,
                retention_days,
                owner: owner.to_owned(),
                registered_at_ns,
            },
        );
        id
    }

    pub fn grant_consent(
        &mut self,
        subject_id: &str,
        purpose: &str,
        category: DataCategory,
        granted_at_ns: u64,
        expires_at_ns: Option<u64>,
    ) -> anyhow::Result<String> {
        if !self.subjects.contains_key(subject_id) {
            return Err(anyhow::Error::new(PrivacyError::SubjectNotFound(subject_id.to_owned())));
        }
        self.consent_seq += 1;
        let id = format!("cons-{:03}", self.consent_seq);
        self.consents.insert(
            id.clone(),
            ConsentGrant {
                id: id.clone(),
                subject_id: subject_id.to_owned(),
                purpose: purpose.to_owned(),
                category,
                granted_at_ns,
                expires_at_ns,
                status: ConsentStatus::Granted,
            },
        );
        Ok(id)
    }

    pub fn revoke_consent(&mut self, consent_id: &str, at_ns: u64) -> anyhow::Result<()> {
        let consent = self
            .consents
            .get_mut(consent_id)
            .ok_or_else(|| anyhow::Error::new(PrivacyError::ConsentNotFound(consent_id.to_owned())))?;
        consent.status = ConsentStatus::Withdrawn;
        consent.expires_at_ns = Some(at_ns);
        Ok(())
    }

    pub fn add_retention_rule(
        &mut self,
        category: DataCategory,
        max_retention_days: u64,
        enforced_at_ns: u64,
    ) -> String {
        self.rule_seq += 1;
        let id = format!("rule-{:03}", self.rule_seq);
        self.rules.push(RetentionRule {
            id,
            category,
            max_retention_days,
            enforced_at_ns,
        });
        self.rules.last().unwrap().id.clone()
    }

    pub fn submit_data_request(
        &mut self,
        subject_id: &str,
        kind: DataRequestKind,
        requested_at_ns: u64,
    ) -> anyhow::Result<String> {
        if !self.subjects.contains_key(subject_id) {
            return Err(anyhow::Error::new(PrivacyError::SubjectNotFound(subject_id.to_owned())));
        }
        self.request_seq += 1;
        let id = format!("req-{:03}", self.request_seq);
        self.requests.insert(
            id.clone(),
            DataRequest {
                id: id.clone(),
                subject_id: subject_id.to_owned(),
                kind,
                status: DataRequestStatus::Open,
                requested_at_ns,
                fulfilled_at_ns: None,
            },
        );
        Ok(id)
    }

    pub fn set_request_in_progress(&mut self, request_id: &str) -> anyhow::Result<()> {
        let req = self
            .requests
            .get_mut(request_id)
            .ok_or_else(|| anyhow::Error::new(PrivacyError::RequestNotFound(request_id.to_owned())))?;
        if req.status == DataRequestStatus::Open {
            req.status = DataRequestStatus::InProgress;
        }
        Ok(())
    }

    pub fn fulfill_request(&mut self, request_id: &str, fulfilled_at_ns: u64) -> anyhow::Result<()> {
        let req = self
            .requests
            .get_mut(request_id)
            .ok_or_else(|| anyhow::Error::new(PrivacyError::RequestNotFound(request_id.to_owned())))?;
        if req.status == DataRequestStatus::Open || req.status == DataRequestStatus::InProgress {
            req.status = DataRequestStatus::Fulfilled;
            req.fulfilled_at_ns = Some(fulfilled_at_ns);
        }
        Ok(())
    }

    pub fn deny_request(&mut self, request_id: &str) -> anyhow::Result<()> {
        let req = self
            .requests
            .get_mut(request_id)
            .ok_or_else(|| anyhow::Error::new(PrivacyError::RequestNotFound(request_id.to_owned())))?;
        if req.status == DataRequestStatus::Open || req.status == DataRequestStatus::InProgress {
            req.status = DataRequestStatus::Denied;
        }
        Ok(())
    }

    pub fn subject_ids(&self) -> Vec<String> {
        self.subjects.keys().cloned().collect()
    }

    pub fn element_ids(&self) -> Vec<String> {
        self.elements.keys().cloned().collect()
    }

    pub fn consent_ids(&self) -> Vec<String> {
        self.consents.keys().cloned().collect()
    }

    pub fn request_ids(&self) -> Vec<String> {
        self.requests.keys().cloned().collect()
    }

    pub fn subject(&self, id: &str) -> Option<&DataSubject> {
        self.subjects.get(id)
    }

    pub fn element(&self, id: &str) -> Option<&DataElement> {
        self.elements.get(id)
    }

    pub fn consent(&self, id: &str) -> Option<&ConsentGrant> {
        self.consents.get(id)
    }

    pub fn request(&self, id: &str) -> Option<&DataRequest> {
        self.requests.get(id)
    }

    pub fn elements_for_category(&self, category: DataCategory) -> Vec<&DataElement> {
        self.elements.values().filter(|e| e.category == category).collect()
    }

    pub fn active_consents_for_subject(&self, subject_id: &str, at_ns: u64) -> Vec<&ConsentGrant> {
        self.consents
            .values()
            .filter(|c| c.subject_id == subject_id && c.is_active(at_ns))
            .collect()
    }

    pub fn max_allowed_retention(&self, category: DataCategory) -> Option<u64> {
        self.rules
            .iter()
            .filter(|r| r.category == category)
            .map(|r| r.max_retention_days)
            .min()
    }

    pub fn inventory(&self) -> DataInventory {
        DataInventory {
            subjects: self.subjects.values().cloned().collect(),
            elements: self.elements.values().cloned().collect(),
            consents: self.consents.values().cloned().collect(),
        }
    }

    pub fn analyze(&self, at_ns: u64) -> PrivacyReport {
        let total_subjects = self.subjects.len();
        let with_active_consent = self
            .subjects
            .keys()
            .filter(|sid| !self.active_consents_for_subject(sid, at_ns).is_empty())
            .count();
        let consent_coverage = if total_subjects == 0 {
            0.0
        } else {
            with_active_consent as f64 / total_subjects as f64
        };

        let mut consent_issues = Vec::new();
        for consent in self.consents.values() {
            if consent.is_expired(at_ns) {
                consent_issues.push(ConsentIssue {
                    subject_id: consent.subject_id.clone(),
                    purpose: consent.purpose.clone(),
                    category: consent.category.clone(),
                    status: ConsentStatus::Expired,
                });
            } else if consent.status == ConsentStatus::Withdrawn {
                consent_issues.push(ConsentIssue {
                    subject_id: consent.subject_id.clone(),
                    purpose: consent.purpose.clone(),
                    category: consent.category.clone(),
                    status: ConsentStatus::Withdrawn,
                });
            }
        }

        let mut retention_violations = Vec::new();
        for element in self.elements.values() {
            if let Some(allowed) = self.max_allowed_retention(element.category.clone()) {
                if element.retention_days > allowed {
                    retention_violations.push(RetentionViolation {
                        element_id: element.id.clone(),
                        category: element.category.clone(),
                        retention_days: element.retention_days,
                        allowed_days: allowed,
                    });
                }
            }
        }

        let mut findings = Vec::new();
        if total_subjects > 0 && with_active_consent == 0 {
            findings.push(PrivacyFinding {
                severity: PrivacySeverity::High,
                message: "no data subject has active consent".to_owned(),
            });
        }
        if with_active_consent < total_subjects && total_subjects > 0 {
            findings.push(PrivacyFinding {
                severity: PrivacySeverity::Medium,
                message: format!("consent coverage is {:.1}%", consent_coverage * 100.0),
            });
        }
        for issue in &consent_issues {
            findings.push(PrivacyFinding {
                severity: PrivacySeverity::Medium,
                message: format!(
                    "consent for subject {} ({}) is {}",
                    issue.subject_id, issue.purpose, issue.status
                ),
            });
        }
        for violation in &retention_violations {
            findings.push(PrivacyFinding {
                severity: PrivacySeverity::Critical,
                message: format!(
                    "element {} retains {}d beyond allowed {}d",
                    violation.element_id, violation.retention_days, violation.allowed_days
                ),
            });
        }
        let open_requests = self
            .requests
            .values()
            .filter(|r| r.status == DataRequestStatus::Open || r.status == DataRequestStatus::InProgress)
            .count();
        if open_requests > 0 {
            findings.push(PrivacyFinding {
                severity: PrivacySeverity::Low,
                message: format!("{} data subject requests outstanding", open_requests),
            });
        }

        let active_consent_count = self.consents.values().filter(|c| c.is_active(at_ns)).count();
        let expired_consent_count = self.consents.values().filter(|c| c.is_expired(at_ns)).count();
        let withdrawn_consent_count = self
            .consents
            .values()
            .filter(|c| c.status == ConsentStatus::Withdrawn)
            .count();

        PrivacyReport {
            policy_version: self.policy_version.clone(),
            subject_count: total_subjects,
            element_count: self.elements.len(),
            consent_count: self.consents.len(),
            active_consent_count,
            expired_consent_count,
            withdrawn_consent_count,
            open_requests,
            consent_coverage,
            consent_issues,
            retention_violations,
            findings,
            generated_at_ns: at_ns,
        }
    }

    pub fn to_event_records(&self, session_id: &str) -> Vec<EventRecord> {
        let mut records = Vec::new();
        let mut sequence = 0u64;
        for (i, subject) in self.subjects.values().enumerate() {
            records.push(event_record(
                session_id,
                &format!("privacy.subject.{:03}", i + 1),
                "privacy.subject",
                &subject,
                &mut sequence,
            ));
        }
        for (i, element) in self.elements.values().enumerate() {
            records.push(event_record(
                session_id,
                &format!("privacy.element.{:03}", i + 1),
                "privacy.element",
                &element,
                &mut sequence,
            ));
        }
        for (i, consent) in self.consents.values().enumerate() {
            records.push(event_record(
                session_id,
                &format!("privacy.consent.{:03}", i + 1),
                "privacy.consent",
                &consent,
                &mut sequence,
            ));
        }
        records
    }

    pub fn report_to_event_records(&self, session_id: &str, report: &PrivacyReport) -> Vec<EventRecord> {
        let mut records = Vec::new();
        let mut sequence = 0u64;
        records.push(event_record(
            session_id,
            "privacy.report.001",
            "privacy.report",
            report,
            &mut sequence,
        ));
        for (i, violation) in report.retention_violations.iter().enumerate() {
            records.push(event_record(
                session_id,
                &format!("privacy.violation.{:03}", i + 1),
                "privacy.violation",
                violation,
                &mut sequence,
            ));
        }
        records
    }
}

fn event_record(
    session_id: &str,
    event_id: &str,
    kind: &str,
    payload: &impl Serialize,
    sequence: &mut u64,
) -> EventRecord {
    let payload_json: serde_json::Value = serde_json::to_value(payload).unwrap_or_default();
    let hash = blake3::hash(format!("{}:{}:{}", session_id, kind, payload_json).as_bytes())
        .to_hex()
        .to_string();
    *sequence += 1;
    EventRecord {
        session_id: session_id.to_owned(),
        event_id: event_id.to_owned(),
        sequence: *sequence,
        kind: kind.to_owned(),
        source: "privacy".to_string(),
        severity: "info".to_string(),
        summary: String::new(),
        payload_json,
        timestamp_ns: 0,
        hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_manager() -> PrivacyManager {
        let mut mgr = PrivacyManager::new("1.0.0");
        mgr.add_subject("alice", "eu", 1_000);
        mgr.add_subject("bob", "us", 2_000);
        mgr
    }

    #[test]
    fn test_add_subject_ids_increment() {
        let mut mgr = PrivacyManager::new("1.0.0");
        let a = mgr.add_subject("alice", "eu", 0);
        let b = mgr.add_subject("bob", "us", 0);
        assert_eq!(a, "sub-001");
        assert_eq!(b, "sub-002");
    }

    #[test]
    fn test_register_element() {
        let mut mgr = PrivacyManager::new("1.0.0");
        let id = mgr.register_element("email", DataCategory::Personal, 30, "auth-svc", 1_000);
        assert_eq!(id, "elem-001");
        assert_eq!(mgr.element(&id).unwrap().retention_days, 30);
    }

    #[test]
    fn test_grant_consent_success() {
        let mut mgr = base_manager();
        let id = mgr
            .grant_consent("sub-001", "analytics", DataCategory::Behavioral, 1_000, None)
            .unwrap();
        assert_eq!(id, "cons-001");
        assert!(mgr.consent(&id).unwrap().is_active(5_000));
    }

    #[test]
    fn test_grant_consent_unknown_subject_errors() {
        let mut mgr = base_manager();
        let result = mgr.grant_consent("sub-999", "analytics", DataCategory::Behavioral, 1_000, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_consent_expiration() {
        let mut mgr = base_manager();
        mgr.grant_consent("sub-001", "analytics", DataCategory::Behavioral, 1_000, Some(2_000))
            .unwrap();
        let consent = mgr.consent("cons-001").unwrap();
        assert!(consent.is_active(2_000));
        assert!(!consent.is_active(2_001));
        assert!(consent.is_expired(2_001));
    }

    #[test]
    fn test_revoke_consent() {
        let mut mgr = base_manager();
        mgr.grant_consent("sub-001", "analytics", DataCategory::Behavioral, 1_000, None)
            .unwrap();
        mgr.revoke_consent("cons-001", 5_000).unwrap();
        let consent = mgr.consent("cons-001").unwrap();
        assert_eq!(consent.status, ConsentStatus::Withdrawn);
        assert!(!consent.is_active(6_000));
    }

    #[test]
    fn test_retention_rule_enforced() {
        let mut mgr = base_manager();
        mgr.register_element("logs", DataCategory::Technical, 500, "ingest", 1_000);
        mgr.add_retention_rule(DataCategory::Technical, 30, 1_000);
        let violations = mgr.analyze(2_000).retention_violations;
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].element_id, "elem-001");
        assert_eq!(violations[0].allowed_days, 30);
    }

    #[test]
    fn test_retention_compliant() {
        let mut mgr = base_manager();
        mgr.register_element("logs", DataCategory::Technical, 7, "ingest", 1_000);
        mgr.add_retention_rule(DataCategory::Technical, 30, 1_000);
        let violations = mgr.analyze(2_000).retention_violations;
        assert!(violations.is_empty());
    }

    #[test]
    fn test_request_lifecycle() {
        let mut mgr = base_manager();
        let req = mgr
            .submit_data_request("sub-001", DataRequestKind::Access, 1_000)
            .unwrap();
        assert_eq!(req, "req-001");
        mgr.set_request_in_progress(&req).unwrap();
        assert_eq!(mgr.request(&req).unwrap().status, DataRequestStatus::InProgress);
        mgr.fulfill_request(&req, 9_000).unwrap();
        assert_eq!(mgr.request(&req).unwrap().status, DataRequestStatus::Fulfilled);
        assert_eq!(mgr.request(&req).unwrap().fulfilled_at_ns, Some(9_000));
    }

    #[test]
    fn test_submit_request_unknown_subject_errors() {
        let mut mgr = base_manager();
        let result = mgr.submit_data_request("sub-999", DataRequestKind::Access, 1_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_deny_request() {
        let mut mgr = base_manager();
        let req = mgr
            .submit_data_request("sub-001", DataRequestKind::Deletion, 1_000)
            .unwrap();
        mgr.deny_request(&req).unwrap();
        assert_eq!(mgr.request(&req).unwrap().status, DataRequestStatus::Denied);
    }

    #[test]
    fn test_consent_coverage_ratio() {
        let mut mgr = base_manager();
        let report = mgr.analyze(5_000);
        assert_eq!(report.consent_coverage, 0.0);
        mgr.grant_consent("sub-001", "analytics", DataCategory::Behavioral, 1_000, None)
            .unwrap();
        let report = mgr.analyze(5_000);
        assert_eq!(report.consent_coverage, 0.5);
    }

    #[test]
    fn test_analyze_finds_issues() {
        let mut mgr = base_manager();
        mgr.register_element("email", DataCategory::Personal, 400, "auth", 1_000);
        mgr.add_retention_rule(DataCategory::Personal, 90, 1_000);
        mgr.grant_consent("sub-001", "analytics", DataCategory::Behavioral, 1_000, Some(2_000))
            .unwrap();
        let report = mgr.analyze(3_000);
        assert!(report.findings.iter().any(|f| f.severity == PrivacySeverity::Critical));
        assert!(report.findings.iter().any(|f| f.message.contains("expired")));
        assert_eq!(report.consent_issues.len(), 1);
        assert_eq!(report.expired_consent_count, 1);
    }

    #[test]
    fn test_open_request_finding() {
        let mut mgr = base_manager();
        mgr.submit_data_request("sub-001", DataRequestKind::Portability, 1_000)
            .unwrap();
        let report = mgr.analyze(2_000);
        assert_eq!(report.open_requests, 1);
        assert!(report.findings.iter().any(|f| f.message.contains("outstanding")));
    }

    #[test]
    fn test_elements_for_category() {
        let mut mgr = base_manager();
        mgr.register_element("email", DataCategory::Personal, 30, "auth", 1_000);
        mgr.register_element("pulse", DataCategory::Health, 60, "wearable", 1_000);
        mgr.register_element("name", DataCategory::Personal, 30, "auth", 1_000);
        assert_eq!(mgr.elements_for_category(DataCategory::Personal).len(), 2);
        assert_eq!(mgr.elements_for_category(DataCategory::Health).len(), 1);
    }

    #[test]
    fn test_event_records_generated() {
        let mut mgr = base_manager();
        mgr.register_element("email", DataCategory::Personal, 30, "auth", 1_000);
        mgr.grant_consent("sub-001", "analytics", DataCategory::Behavioral, 1_000, None)
            .unwrap();
        let records = mgr.to_event_records("sess-1");
        assert_eq!(records.len(), 4);
        assert!(records.iter().all(|r| r.session_id == "sess-1"));
        let kinds: Vec<&str> = records.iter().map(|r| r.kind.as_str()).collect();
        assert!(kinds.contains(&"privacy.subject"));
        assert!(kinds.contains(&"privacy.element"));
        assert!(kinds.contains(&"privacy.consent"));
        assert!(records.iter().all(|r| !r.hash.is_empty()));
    }

    #[test]
    fn test_report_to_event_records() {
        let mut mgr = base_manager();
        mgr.register_element("logs", DataCategory::Technical, 500, "ingest", 1_000);
        mgr.add_retention_rule(DataCategory::Technical, 30, 1_000);
        let report = mgr.analyze(2_000);
        let records = mgr.report_to_event_records("sess-1", &report);
        assert_eq!(records.len(), 2);
        assert!(records[0].kind == "privacy.report");
        assert!(records[1].kind == "privacy.violation");
    }
}
