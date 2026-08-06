use std::time::{Duration, Instant};

use runlens_core::model::{Event, EventSource, PrivacyClassification, Severity};
use serde::Serialize;
use tokio::time::sleep;

use crate::dispatch::{monotonic_now_ns, Dispatcher};

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
        let stop = stop_flag.clone();
        let dispatcher_for_task = dispatcher.clone();
        tokio::spawn(async move {
            loop {
                if stop.load(std::sync::atomic::Ordering::Relaxed) {
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
                    let _ = dispatcher_for_task.emit(event);
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
        self.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
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
    let mut s = ProfileSample {
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
    #[cfg(target_family = "windows")]
    {
        win_sample(&mut s);
    }
    s
}

#[cfg(target_family = "windows")]
fn win_sample(s: &mut ProfileSample) {
    let pid = std::process::id();
    let script = format!(
        "$p = Get-Process -Id {}; \
         $os = Get-CimInstance Win32_OperatingSystem; \
         [PSCustomObject]@{{ \
             ws=$p.WorkingSet64; vm=$p.VirtualMemorySize64; \
             pf=$p.PrivateMemorySize64; \
             mt=$os.TotalVisibleMemorySize; ma=$os.FreePhysicalMemory \
         }} | ConvertTo-Json -Compress",
        pid
    );
    let Ok(out) = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
    else {
        return;
    };
    let Ok(text) = String::from_utf8(out.stdout) else {
        return;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    s.rss_bytes = v.get("ws").and_then(|x| x.as_u64());
    s.virtual_bytes = v.get("vm").and_then(|x| x.as_u64());
    s.page_faults = v.get("pf").and_then(|x| x.as_u64());
    s.system_mem_total = v.get("mt").and_then(|x| x.as_u64()).map(|kb| kb * 1024);
    s.system_mem_avail = v.get("ma").and_then(|x| x.as_u64()).map(|kb| kb * 1024);
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
