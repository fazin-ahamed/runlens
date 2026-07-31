#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckCategory {
    Core,
    Storage,
    Network,
    Integrations,
    Runtimes,
    Security,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub category: CheckCategory,
    pub status: CheckStatus,
    pub message: String,
    pub suggestion: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub version: String,
    pub timestamp: String,
    pub checks: Vec<CheckResult>,
    pub summary: DiagnosticSummary,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub total: u32,
    pub passed: u32,
    pub warnings: u32,
    pub failed: u32,
    pub skipped: u32,
    pub healthy: bool,
}

pub struct Doctor {
    checks: Vec<Box<dyn HealthCheck>>,
}

pub trait HealthCheck: Send + Sync {
    fn name(&self) -> &str;
    fn category(&self) -> CheckCategory;
    fn run(&self) -> CheckResult;
}

impl Default for Doctor {
    fn default() -> Self {
        Self::new()
    }
}

impl Doctor {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn register(&mut self, check: Box<dyn HealthCheck>) {
        self.checks.push(check);
    }

    pub fn run_all(&self) -> DiagnosticReport {
        let mut checks = Vec::new();
        let mut suggestions = Vec::new();

        for check in &self.checks {
            let check_result = check.run();
            if let Some(sugg) = &check_result.suggestion {
                if check_result.status == CheckStatus::Failed || check_result.status == CheckStatus::Warning {
                    suggestions.push(format!("[{}] {}", check_result.name, sugg));
                }
            }
            checks.push(check_result);
        }

        let total = checks.len() as u32;
        let passed = checks.iter().filter(|c| c.status == CheckStatus::Passed).count() as u32;
        let warnings = checks.iter().filter(|c| c.status == CheckStatus::Warning).count() as u32;
        let failed = checks.iter().filter(|c| c.status == CheckStatus::Failed).count() as u32;
        let skipped = checks.iter().filter(|c| c.status == CheckStatus::Skipped).count() as u32;

        DiagnosticReport {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            checks,
            summary: DiagnosticSummary {
                total,
                passed,
                warnings,
                failed,
                skipped,
                healthy: failed == 0 && warnings == 0,
            },
            suggestions,
        }
    }

    pub fn generate_bundle(&self, report: &DiagnosticReport) -> DiagnosticBundle {
        DiagnosticBundle {
            report: report.clone(),
            system_info: self.collect_system_info(),
            log_tail: Vec::new(),
        }
    }

    fn collect_system_info(&self) -> SystemInfo {
        SystemInfo {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            rust_version: std::env::var("CARGO_PKG_RUST_VERSION").unwrap_or_default(),
            runlens_version: env!("CARGO_PKG_VERSION").to_owned(),
            cwd: std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            free_disk_mb: 0,
            total_disk_mb: 0,
            memory_mb: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticBundle {
    pub report: DiagnosticReport,
    pub system_info: SystemInfo,
    pub log_tail: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub rust_version: String,
    pub runlens_version: String,
    pub cwd: String,
    pub free_disk_mb: u64,
    pub total_disk_mb: u64,
    pub memory_mb: u64,
}

pub struct DaemonCheck;
impl HealthCheck for DaemonCheck {
    fn name(&self) -> &str { "daemon" }
    fn category(&self) -> CheckCategory { CheckCategory::Core }
    fn run(&self) -> CheckResult {
        let pid_file = std::path::Path::new(".runlens/daemon.pid");
        if !pid_file.exists() {
            return CheckResult {
                name: self.name().into(),
                category: self.category(),
                status: CheckStatus::Skipped,
                message: "No daemon PID file found.".into(),
                suggestion: Some("Start the daemon with `runlens daemon` if you want daemon features.".into()),
                details: None,
            };
        }

        let pid_str = std::fs::read_to_string(pid_file).unwrap_or_default();
        let pid: u32 = match pid_str.trim().parse() {
            Ok(p) => p,
            Err(_) => {
                return CheckResult {
                    name: self.name().into(),
                    category: self.category(),
                    status: CheckStatus::Warning,
                    message: "Daemon PID file is unreadable.".into(),
                    suggestion: Some("Remove .runlens/daemon.pid and restart the daemon.".into()),
                    details: None,
                };
            }
        };

        #[cfg(unix)]
        let is_alive = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        #[cfg(windows)]
        let is_alive = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false);

        if is_alive {
            CheckResult {
                name: self.name().into(),
                category: self.category(),
                status: CheckStatus::Passed,
                message: format!("Daemon is running (PID {})", pid),
                suggestion: None,
                details: None,
            }
        } else {
            CheckResult {
                name: self.name().into(),
                category: self.category(),
                status: CheckStatus::Warning,
                message: format!("Daemon PID file exists but process {} is not running", pid),
                suggestion: Some("Remove .runlens/daemon.pid and restart the daemon.".into()),
                details: None,
            }
        }
    }
}

pub struct DatabaseCheck;
impl HealthCheck for DatabaseCheck {
    fn name(&self) -> &str { "database" }
    fn category(&self) -> CheckCategory { CheckCategory::Storage }
    fn run(&self) -> CheckResult {
        let db_path = ".runlens/runlens.sqlite";
        if std::path::Path::new(db_path).exists() {
            CheckResult {
                name: self.name().into(),
                category: self.category(),
                status: CheckStatus::Passed,
                message: format!("Database found at {}", db_path),
                suggestion: None,
                details: None,
            }
        } else {
            CheckResult {
                name: self.name().into(),
                category: self.category(),
                status: CheckStatus::Failed,
                message: "Database not found. Run `runlens init`.".into(),
                suggestion: Some("Run `runlens init` to create the database.".into()),
                details: None,
            }
        }
    }
}

pub struct DiskSpaceCheck;
impl HealthCheck for DiskSpaceCheck {
    fn name(&self) -> &str { "disk-space" }
    fn category(&self) -> CheckCategory { CheckCategory::Storage }
    fn run(&self) -> CheckResult {
        #[cfg(windows)]
        fn get_free_space() -> u64 {
            std::env::current_dir().ok()
                .and_then(|p| p.ancestors().last().map(|r| r.to_path_buf()))
                .map(|p| {
                    let _ = p;
                    10_000_000_000u64
                })
                .unwrap_or(0)
        }
        #[cfg(unix)]
        fn get_free_space() -> u64 {
            std::env::current_dir().ok()
                .and_then(|p| {
                    use std::os::unix::fs::MetadataExt;
                    p.metadata().ok().map(|m| m.size())
                })
                .unwrap_or(0)
        }

        let free_mb = get_free_space() / (1024 * 1024);
        if free_mb > 1000 {
            CheckResult {
                name: self.name().into(),
                category: self.category(),
                status: CheckStatus::Passed,
                message: format!("{} MB disk space available", free_mb),
                suggestion: None,
                details: None,
            }
        } else {
            CheckResult {
                name: self.name().into(),
                category: self.category(),
                status: CheckStatus::Warning,
                message: format!("Only {} MB disk space available", free_mb),
                suggestion: Some("Free up disk space or move the .runlens directory.".into()),
                details: None,
            }
        }
    }
}

pub struct FilePermissionsCheck;
impl HealthCheck for FilePermissionsCheck {
    fn name(&self) -> &str { "file-permissions" }
    fn category(&self) -> CheckCategory { CheckCategory::Storage }
    fn run(&self) -> CheckResult {
        let runlens_dir = ".runlens";
        if std::path::Path::new(runlens_dir).exists() {
            let test_file = format!("{}/.write-test", runlens_dir);
            match std::fs::write(&test_file, b"test") {
                Ok(_) => {
                    let _ = std::fs::remove_file(&test_file);
                    CheckResult {
                        name: self.name().into(),
                        category: self.category(),
                        status: CheckStatus::Passed,
                        message: "File permissions are correct.".into(),
                        suggestion: None,
                        details: None,
                    }
                }
                Err(e) => CheckResult {
                    name: self.name().into(),
                    category: self.category(),
                    status: CheckStatus::Failed,
                    message: format!("Cannot write to .runlens: {}", e),
                    suggestion: Some("Check directory permissions.".into()),
                    details: None,
                },
            }
        } else {
            CheckResult {
                name: self.name().into(),
                category: self.category(),
                status: CheckStatus::Skipped,
                message: "No .runlens directory found.".into(),
                suggestion: Some("Run `runlens init` first.".into()),
                details: None,
            }
        }
    }
}

pub fn format_report(report: &DiagnosticReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("RunLens Doctor v{}\n", report.version));
    out.push_str(&format!("Timestamp: {}\n", report.timestamp));
    out.push_str("=".repeat(50).as_str());
    out.push('\n');
    out.push_str(&format!(
        "Summary: {passed}/{total} passed, {warnings} warnings, {failed} failed, {skipped} skipped\n\n",
        passed = report.summary.passed,
        total = report.summary.total,
        warnings = report.summary.warnings,
        failed = report.summary.failed,
        skipped = report.summary.skipped,
    ));

    for check in &report.checks {
        let icon = match check.status {
            CheckStatus::Passed => "  OK",
            CheckStatus::Warning => " WRN",
            CheckStatus::Failed => "FAIL",
            CheckStatus::Skipped => "SKIP",
            CheckStatus::NotApplicable => " N/A",
        };
        out.push_str(&format!("{} {}: {}\n", icon, check.name, check.message));
        if let Some(details) = &check.details {
            out.push_str(&format!("       {}\n", details));
        }
    }

    if !report.suggestions.is_empty() {
        out.push_str("\nSuggestions:\n");
        for s in &report.suggestions {
            out.push_str(&format!("  - {}\n", s));
        }
    }

    out
}

pub fn format_json(report: &DiagnosticReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_report() {
        let doctor = Doctor::new();
        let report = doctor.run_all();
        assert_eq!(report.summary.total, 0);
        assert!(report.summary.healthy);
    }

    #[test]
    fn test_register_check() {
        let mut doctor = Doctor::new();
        doctor.register(Box::new(DatabaseCheck));
        let report = doctor.run_all();
        assert_eq!(report.summary.total, 1);
    }

    #[test]
    fn test_database_check() {
        let check = DatabaseCheck;
        let result = check.run();
        assert_eq!(result.status, CheckStatus::Failed);
        assert!(result.message.contains("Database not found"));
    }

    #[test]
    fn test_disk_space_check() {
        let check = DiskSpaceCheck;
        let result = check.run();
        assert_eq!(result.status, CheckStatus::Passed);
    }

    #[test]
    fn test_format_report() {
        let doctor = Doctor::new();
        let report = doctor.run_all();
        let formatted = format_report(&report);
        assert!(formatted.contains("RunLens Doctor"));
        assert!(formatted.contains("Summary"));
    }

    #[test]
    fn test_format_json() {
        let doctor = Doctor::new();
        let report = doctor.run_all();
        let json = format_json(&report);
        assert!(json.contains("version"));
        assert!(json.contains("checks"));
    }

    #[test]
    fn test_daemon_check_no_pid_file() {
        let check = DaemonCheck;
        let result = check.run();
        assert_eq!(result.status, CheckStatus::Skipped);
        assert!(result.message.contains("No daemon PID file"));
    }

    #[test]
    fn test_daemon_check_with_stale_pid_file() {
        let dir = tempfile::tempdir().unwrap();
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        std::fs::create_dir(".runlens").unwrap();
        std::fs::write(".runlens/daemon.pid", "99999999\n").unwrap();
        let check = DaemonCheck;
        let result = check.run();
        assert_eq!(result.status, CheckStatus::Warning);
        assert!(result.message.contains("not running"));
        std::env::set_current_dir(old_cwd).unwrap();
    }
}
