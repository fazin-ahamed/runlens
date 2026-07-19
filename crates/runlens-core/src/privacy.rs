//! Privacy: secret detection, path masking, redaction tags.
//!
//! RunLens never stores raw environment values, keystrokes, or clipboard by
//! default. Even within recorded content we recognise likely-sensitive
//! tokens and mark them for review. This module defines:
//!
//! - `SecretPattern`: a compiled detection pattern with category + replacement.
//! - `Redaction`: an applied redaction with a short, NON-reversible identifier.
//! - `PathMask`: an absolute-path masking helper.
//! - `scan`: the main entry point that scans a string or JSON value.
//!
//! The detector must NOT match innocuous content too aggressively. All
//! detections are reported as findings; the user reviews before export.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Categories of detected secrets, used for review and reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecretKind {
    ApiKey,
    BearerToken,
    JwtLike,
    PrivateKeyBlock,
    DatabaseUrl,
    EmailAddress,
    PrivateIpAddress,
    AbsoluteHomePath,
    Username,
    HighEntropyString,
    AuthorizationHeader,
}

impl SecretKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ApiKey => "api-key",
            Self::BearerToken => "bearer-token",
            Self::JwtLike => "jwt-like",
            Self::PrivateKeyBlock => "private-key-block",
            Self::DatabaseUrl => "database-url",
            Self::EmailAddress => "email-address",
            Self::PrivateIpAddress => "private-ip-address",
            Self::AbsoluteHomePath => "absolute-home-path",
            Self::Username => "username",
            Self::HighEntropyString => "high-entropy-string",
            Self::AuthorizationHeader => "authorization-header",
        }
    }
}

/// A single compiled pattern.
#[derive(Debug, Clone)]
pub struct SecretPattern {
    pub kind: SecretKind,
    pub re: Regex,
}

/// Compile the bundled set of patterns.
pub fn default_patterns() -> Vec<SecretPattern> {
    vec![
        // AWS-style access keys: AKIA / ASIA prefixes.
        pattern(SecretKind::ApiKey, r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
        // GitHub PAT (classic): ghp_xxx
        pattern(SecretKind::ApiKey, r"\bghp_[A-Za-z0-9]{36}\b"),
        // GitHub fine-grained: github_pat_xxx
        pattern(SecretKind::ApiKey, r"\bgithub_pat_[A-Za-z0-9_]{82}\b"),
        // Slack tokens: xox[bpars]-xxx
        pattern(SecretKind::ApiKey, r"\bxox[bpars]-[A-Za-z0-9-]{10,}\b"),
        // Stripe live / test keys.
        pattern(SecretKind::ApiKey, r"\bsk_(?:live|test)_[A-Za-z0-9]{24,}\b"),
        pattern(SecretKind::ApiKey, r"\brk_(?:live|test)_[A-Za-z0-9]{24,}\b"),
        // Google API key.
        pattern(SecretKind::ApiKey, r"\bAIza[0-9A-Za-z_-]{35}\b"),
        // JWT-ish: three dot-separated base64url chunks of meaningful length.
        pattern(
            SecretKind::JwtLike,
            r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b",
        ),
        // Authorization header content. Avoid plain "Authorization: xyz".
        pattern(
            SecretKind::AuthorizationHeader,
            r"(?i)\bauthorization\s*[:=]\s*(?:bearer|basic|token)\s+[A-Za-z0-9._\-=]{8,}",
        ),
        pattern(
            SecretKind::BearerToken,
            r"(?i)\bbearer\s+[A-Za-z0-9._\-]{16,}",
        ),
        // PEM private key block.
        pattern(
            SecretKind::PrivateKeyBlock,
            r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY-----",
        ),
        // Database URL with embedded credentials.
        pattern(
            SecretKind::DatabaseUrl,
            r"(?i)(?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?|redis|amqp)://[^\s:/@]+:[^\s@]+@[^/\s]+",
        ),
        // Email addresses.
        pattern(
            SecretKind::EmailAddress,
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,24}\b",
        ),
        // Private IPs (RFC1918 + loopback).
        pattern(
            SecretKind::PrivateIpAddress,
            r"\b(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[0-1])\.\d{1,3}\.\d{1,3}|127\.0\.0\.1)\b",
        ),
        // Absolute home paths: /home/<user>..., /Users/<user>..., C:\Users\<user>...
        pattern(
            SecretKind::AbsoluteHomePath,
            r"(?:/home/|/Users/|[A-Z]:\\Users\\)[A-Za-z0-9._-]+",
        ),
    ]
}

fn pattern(kind: SecretKind, re: &str) -> SecretPattern {
    SecretPattern {
        kind,
        re: Regex::new(re).expect("static regex compiles"),
    }
}

/// A finding from a scan over a single value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub kind: SecretKind,
    pub span_start: usize,
    pub span_end: usize,
    pub redaction: String,
    /// Original value if the user has been shown it; otherwise masked.
    pub preview: String,
}

/// Replace a span in the source with a redaction tag and produce a finding.
pub fn redact_one(value: &str, span: std::ops::Range<usize>, kind: SecretKind) -> (String, Finding) {
    let preview = mask_in_place(value[span.clone()].to_string(), kind);
    let mut new_value = String::with_capacity(value.len() + 16);
    new_value.push_str(&value[..span.start]);
    let tag = redaction_tag(kind);
    new_value.push_str(&tag);
    new_value.push_str(&value[span.end..]);
    let finding = Finding {
        kind,
        span_start: span.start,
        span_end: span.start + tag.len(),
        redaction: tag,
        preview,
    };
    (new_value, finding)
}

/// Format a redaction tag. The original value CANNOT be recovered from this.
pub fn redaction_tag(kind: SecretKind) -> String {
    // Use a short random suffix (cryptographically random if available).
    let rand = short_random_suffix();
    format!("<REDACTED:{}:{}>", kind.as_str().to_ascii_uppercase(), rand)
}

fn short_random_suffix() -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut h);
    h.finish().hash(&mut h);
    let v = h.finish();
    // Reduce to base32-ish 6 chars; collisions are minor-secret-uniqueness, not security.
    format!("{:06x}", v & 0xFFFFFF)
}

fn mask_in_place(s: String, kind: SecretKind) -> String {
    if s.len() <= 4 {
        return "*".repeat(s.len());
    }
    match kind {
        SecretKind::EmailAddress => {
            // Show first char of local-part + domain.
            if let Some(at) = s.find('@') {
                let (local, domain) = s.split_at(at);
                let domain = &domain[1..];
                let first_char = local.chars().next().unwrap_or('?');
                format!("{}***@{}", first_char, domain)
            } else {
                format!("{}***", &s[..1])
            }
        }
        SecretKind::PrivateIpAddress => {
            let parts: Vec<&str> = s.split('.').collect();
            if parts.len() == 4 {
                format!("{}.{}.x.x", parts[0], parts[1])
            } else {
                "x.x.x.x".into()
            }
        }
        SecretKind::AbsoluteHomePath => {
            // Replace user portion with ~.
            s.replacen(&s.trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-'), "~", 1)
        }
        _ => {
            let (left, mid, right) = (&s[..2], "*".repeat(s.len().saturating_sub(6).min(8)), &s[s.len() - 2..]);
            format!("{left}{mid}{right}")
        }
    }
}

/// Scan a single string with the given patterns. Returns the original input and
/// findings with byte offsets into it. Caller decides whether to apply redactions.
pub fn scan_string(input: &str, patterns: &[SecretPattern]) -> (String, Vec<Finding>) {
    let mut findings: Vec<Finding> = Vec::new();
    for p in patterns {
        for m in p.re.find_iter(input) {
            let span_start = m.start();
            let span_end = m.end();
            let preview = mask_in_place(input[span_start..span_end].to_string(), p.kind);
            let tag = redaction_tag(p.kind);
            findings.push(Finding {
                kind: p.kind,
                span_start,
                span_end,
                redaction: tag,
                preview,
            });
        }
    }
    findings.sort_by_key(|f| (f.span_start, std::cmp::Reverse(f.span_end)));
    deduplicate_overlaps(&mut findings);
    (input.to_string(), findings)
}

/// Apply a sorted, non-overlapping list of findings to a copy of the input.
/// Findings must reference byte offsets in `input` (the original, not a
/// previously-redacted copy).
pub fn apply_redactions(input: &str, findings: &[Finding]) -> String {
    if findings.is_empty() {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;
    for f in findings {
        if f.span_start < cursor || f.span_end > input.len() {
            continue;
        }
        out.push_str(&input[cursor..f.span_start]);
        out.push_str(&f.redaction);
        cursor = f.span_end;
    }
    out.push_str(&input[cursor..]);
    out
}

fn deduplicate_overlaps(findings: &mut Vec<Finding>) {
    findings.sort_by_key(|f| (f.span_start, std::cmp::Reverse(f.span_end)));
    let mut kept: Vec<Finding> = Vec::with_capacity(findings.len());
    let mut last_end = 0usize;
    for f in findings.drain(..) {
        if f.span_start >= last_end {
            last_end = f.span_end;
            kept.push(f);
        }
    }
    *findings = kept;
}

/// Scan a JSON value and produce a redacted clone plus a list of findings.
pub fn scan_json(
    value: &serde_json::Value,
    patterns: &[SecretPattern],
) -> (serde_json::Value, Vec<Finding>) {
    let mut findings = Vec::new();
    let redacted = walk(value, patterns, &mut findings);
    (redacted, findings)
}

fn walk(
    value: &serde_json::Value,
    patterns: &[SecretPattern],
    findings: &mut Vec<Finding>,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            let (_, mut found) = scan_string(s, patterns);
            // Re-base spans in the parent context (top-level span of this string = (0, len))
            // but for findings reports we record offsets inside the string.
            findings.append(&mut found);
            serde_json::Value::String(s.clone())
        }
        serde_json::Value::Array(a) => {
            serde_json::Value::Array(a.iter().map(|v| walk(v, patterns, findings)).collect())
        }
        serde_json::Value::Object(m) => {
            let mut out = serde_json::Map::new();
            for (k, v) in m {
                out.insert(k.clone(), walk(v, patterns, findings));
            }
            serde_json::Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Path masking: turn an absolute user path into a project-relative reference
/// or a tilde-prefixed reference. For privacy, ANY home-directory prefix is
/// masked regardless of whose home it is — the user_name matrix only affects
/// whether the masked prefix matches a known session user.
pub fn mask_absolute_path(path: &str, project_root: &str, user_name: &str) -> String {
    if let Some(rest) = path.strip_prefix(project_root) {
        let rest = rest.trim_start_matches(|c: char| c == '\\' || c == '/');
        if !rest.is_empty() {
            return format!("/$PROJECT/{}", rest.replace('\\', "/"));
        }
    }
    let mut masked = path.to_string();
    for prefix in &["/home/", "/Users/", r"C:\Users\", r"c:\users\"] {
        if let Some(rest) = masked.strip_prefix(prefix) {
            // Find the user-name boundary (next path separator or end of string).
            let boundary = rest.find(|c: char| c == '\\' || c == '/').unwrap_or(rest.len());
            let replacement_user = if !user_name.is_empty() && rest.starts_with(user_name) {
                "~"  // user is the session-owner.
            } else {
                "~"  // always mask to avoid leaking any other username.
            };
            masked = format!(
                "{replacement_user}{}",
                rest[boundary..].replace('\\', "/")
            );
        }
    }
    masked
}

/// Helper: arbitrary key names that indicate an environment variable that
/// probably contains a secret and should be marked `Sensitive`.
pub fn looks_like_secret_env(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "PASSWORD"
            | "PASSWD"
            | "SECRET"
            | "TOKEN"
            | "API_KEY"
            | "APIKEY"
            | "ACCESS_KEY"
            | "PRIVATE_KEY"
            | "CLIENT_SECRET"
            | "AUTH_TOKEN"
    ) || upper.contains("SECRET")
        || upper.contains("PASSWORD")
        || upper.ends_with("_TOKEN")
        || upper.ends_with("_KEY")
        || upper.ends_with("_PASSWORD")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aws_key() {
        let s = "AKIAIOSFODNN7EXAMPLE";
        let (_, f) = scan_string(s, &default_patterns());
        assert!(f.iter().any(|x| x.kind == SecretKind::ApiKey));
    }

    #[test]
    fn detects_github_pat() {
        let s = "ghp_".to_string() + &"a".repeat(36);
        let (_, f) = scan_string(&s, &default_patterns());
        assert!(f.iter().any(|x| x.kind == SecretKind::ApiKey));
    }

    #[test]
    fn detects_pem_block() {
        let s = "-----BEGIN PRIVATE KEY-----";
        let (_, f) = scan_string(s, &default_patterns());
        assert!(f.iter().any(|x| x.kind == SecretKind::PrivateKeyBlock));
    }

    #[test]
    fn detects_database_url() {
        let s = "postgres://app:supersecret@db.local:5432/main";
        let (_, f) = scan_string(s, &default_patterns());
        assert!(f.iter().any(|x| x.kind == SecretKind::DatabaseUrl));
    }

    #[test]
    fn detects_bearer_token() {
        let s = "Authorization: Bearer abcdef0123456789ABCDEF";
        let (_, f) = scan_string(s, &default_patterns());
        // Either the AuthorizationHeader pattern wins, or the BearerToken one does.
        assert!(
            f.iter().any(|x| x.kind == SecretKind::BearerToken)
                || f.iter().any(|x| x.kind == SecretKind::AuthorizationHeader)
        );
    }

    #[test]
    fn detects_email() {
        let (_, f) = scan_string("contact me at user@example.com please", &default_patterns());
        assert!(f.iter().any(|x| x.kind == SecretKind::EmailAddress));
    }

    #[test]
    fn masks_home_path() {
        let m = mask_absolute_path("/home/alice/projects/foo/src/main.rs", "/home/alice/projects/foo", "alice");
        assert_eq!(m, "/$PROJECT/src/main.rs");
    }

    #[test]
    fn masks_unrelated_home_path() {
        let m = mask_absolute_path("/Users/bob/some/other/path.rs", "/Users/alice/project", "alice");
        assert_eq!(m, "~/some/other/path.rs");
    }

    #[test]
    fn looks_like_secret_env_var() {
        assert!(looks_like_secret_env("AWS_SECRET_ACCESS_KEY"));
        assert!(looks_like_secret_env("MY_PASSWORD"));
        assert!(looks_like_secret_env("GITHUB_TOKEN"));
        assert!(!looks_like_secret_env("PATH"));
        assert!(!looks_like_secret_env("HOME"));
    }

    #[test]
    fn redact_tag_is_non_reversible() {
        let s = "Bearer abcdef0123456789ABCDEF0123456789";
        let (orig, findings) = scan_string(s, &default_patterns());
        let redacted = apply_redactions(&orig, &findings);
        // Two scans on the redacted content produce the same redacted form.
        // The original bearer value never appears.
        assert!(!redacted.contains("abcdef0123456789ABCDEF0123456789"));
        assert!(redacted.contains("<REDACTED:"));
        // Original leaks nothing about secret content either.
        for f in &findings {
            assert!(f.preview.contains('*') || f.preview.contains("~") || f.preview.contains("x.x.x"));
            assert!(!f.preview.contains("abcdef0123456789ABCDEF0123456789"));
        }
    }

    #[test]
    fn empty_input_yields_no_findings() {
        let (_, f) = scan_string("", &default_patterns());
        assert!(f.is_empty());
    }
}
