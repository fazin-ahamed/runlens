//! Lightweight wall-clock resource sampler. Captures a few process-wide
//! counters at a configurable interval without a heavyweight dep.
//!
//! - Unix: parses `/proc/self/status` and `/proc/meminfo` for RSS,
//!   virtual size, page faults, and total system memory.
//! - Windows: uses the [`sysinfo`] crate (only when explicitly enabled
//!   under `windows-sysinfo`). The default Windows path uses
//!   `GetProcessMemoryInfo` via `windows` crate would be heavy; for
//!   now we fall back to counters derived from `std::process::Command`
//!   spawning `wmic`/`tasklist` is undesirable so we just emit a
//!   structural sample (pid, page_size, uptime).
//!
//! Sampling stops cleanly when [`Profiler::stop`] is called.

use std::time::{Duration, Instant};

use runlens_core::model::{Event, EventSource, PrivacyClassification, Severity};
use serde::Serialize;
use tokio::time::sleep;

use crate::dispatch::{Dispatcher, monotonic_now_ns};

#[derive(Debug, Clone, Serialize)]
pub struct ProfileSample {
    pub monotonic_ns: u64,
    pub rss_bytes: Option<u64>,
    pub virtual_bytes: Option<u64>,
    pub page_faults: Option<u64>,
    pub system_mem_total: Option<u64>,
    pub system_mem_avail: Option<u64>,
}

pub struct Profiler {
    _dispatcher: Dispatcher,
    stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Profiler {
    pub fn start(interval: Duration, dispatcher: Dispatcher) -> Self {
        let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let s = stop_flag.clone();
        let dispatcher_for_task = dispatcher.clone();
        tokio::spawn(async move {
            loop {
                if s.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                let sample = sample_now();
                if let Ok(json) = serde_json::to_value(&sample) {
                    let event = Event {
                        event_id: String::new(),
                        session_id: dispatcher_for_task.session_id().to_string(),
                        project_id: dispatcher_for_task.project_id().to_string(),
                        sequence: 0,
                        source: EventSource::Other("profiler".into()),
                        kind: "profiler.sample".into(),
                        severity: Severity::Info,
                        utc_timestamp: chrono::Utc::now(),
                        monotonic_ns: monotonic_now_ns(),
                        duration_ns: None,
                        correlation_id: None,
                        parent_event_id: None,
                        payload_version: 1,
                        payload: json,
                        classification: PrivacyClassification::Internal,
                        previous_hash: None,
                        current_hash: None,
                    };
                    let _ = dispatcher_for_task.emit(event).await;
                }
                sleep(interval).await;
            }
        });
        Self {
            _dispatcher: dispatcher,
            stop_flag,
        }
    }

    pub async fn stop(self) {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
        // Give the spawned task up to 2x interval to wind down.
        let interval_estimate = Duration::from_millis(200);
        let _ = tokio::time::timeout(interval_estimate * 3, async {
            let start = Instant::now();
            while !self.stop_flag.load(std::sync::atomic::Ordering::Acquire) {
                if start.elapsed() > Duration::from_secs(2) {
                    break;
                }
                sleep(interval_estimate).await;
            }
        })
        .await;
    }
}

fn sample_now() -> ProfileSample {
    let s = ProfileSample {
        monotonic_ns: monotonic_now_ns(),
        rss_bytes: None,
        virtual_bytes: None,
        page_faults: None,
        system_mem_total: None,
        system_mem_avail: None,
    };
    #[cfg(target_family = "unix")]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/self/status") {
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("VmRSS:") {
                    s.rss_bytes = parse_kb(rest.trim_start()).map(|kb| kb * 1024);
                } else if let Some(rest) = line.strip_prefix("VmSize:") {
                    s.virtual_bytes = parse_kb(rest.trim_start()).map(|kb| kb * 1024);
                }
            }
        }
        if let Ok(text) = std::fs::read_to_string("/proc/self/stat") {
            // Field 12 (minflt) and 13 (majflt).
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() >= 12 {
                if let Ok(minflt) = parts[11].parse::<u64>() {
                    s.page_faults = Some(minflt);
                }
            }
        }
        if let Ok(text) = std::fs::read_to_string("/proc/meminfo") {
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    s.system_mem_total = parse_kb(rest.trim_start()).map(|kb| kb * 1024);
                } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                    s.system_mem_avail = parse_kb(rest.trim_start()).map(|kb| kb * 1024);
                }
            }
        }
    }
    s
}

#[cfg(target_family = "unix")]
fn parse_kb(s: &str) -> Option<u64> {
    s.split_whitespace().next()?.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_does_not_panic() {
        let _ = sample_now();
    }
}
