use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::{Config, Event, EventKind as NotifyEventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tracing::warn;

const DEBOUNCE_WINDOW_MS: u64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FsAction {
    Create,
    Modify,
    Remove,
    Rename,
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct FsEvent {
    pub sequence: u64,
    pub action: FsAction,
    pub ext: Option<String>,
    pub size_hint: Option<u64>,
}

pub struct FsWatcher {
    _watcher: RecommendedWatcher,
    pub rx: mpsc::Receiver<FsEvent>,
    _seq: u64,
}

impl FsWatcher {
    pub fn start(roots: &[PathBuf], ignore: &[String]) -> Result<Self> {
        let (raw_tx, raw_rx) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher = RecommendedWatcher::new(
            move |event| {
                let _ = raw_tx.send(event);
            },
            Config::default().with_poll_interval(Duration::from_millis(200)),
        )
        .context("construct notify watcher")?;
        if roots.is_empty() {
            anyhow::bail!("no roots to watch");
        }
        for root in roots {
            watcher
                .watch(root, RecursiveMode::Recursive)
                .with_context(|| format!("watch {:?}", root))?;
        }

        let (tx, rx) = mpsc::channel::<FsEvent>();
        let mut seq: u64 = 0;
        let root_buf = roots[0].to_path_buf();
        let ignore = ignore.to_vec();
        let mut debounced = Debounced::default();
        std::thread::spawn(move || loop {
            match raw_rx.recv() {
                Ok(Ok(event)) => {
                    let Some(out) =
                        debounce(&mut debounced, &event, &root_buf, &ignore, &mut seq)
                    else {
                        continue;
                    };
                    if tx.send(out).is_err() {
                        break;
                    }
                }
                Ok(Err(e)) => warn!(error=%e, "watcher err"),
                Err(_) => break,
            }
        });

        Ok(Self {
            _watcher: watcher,
            rx,
            _seq: seq,
        })
    }

    pub fn try_recv(&self) -> Option<FsEvent> {
        self.rx.try_recv().ok()
    }
}

#[derive(Default, Debug)]
struct Debounced {
    last_seen_at: Option<std::collections::HashMap<PathBuf, Instant>>,
}

fn debounce(
    state: &mut Debounced,
    event: &Event,
    root: &Path,
    ignore: &[String],
    seq: &mut u64,
) -> Option<FsEvent> {
    let mut keep = false;
    let mut action = match event.kind {
        NotifyEventKind::Create(_) => FsAction::Create,
        NotifyEventKind::Modify(_) => FsAction::Modify,
        NotifyEventKind::Remove(_) => FsAction::Remove,
        NotifyEventKind::Any => FsAction::Other,
        _ => return None,
    };
    for path in &event.paths {
        if matches_ignore(path, ignore) || !within_root(path, root) {
            continue;
        }
        let window = state.last_seen_at.get_or_insert_with(Default::default);
        let last_seen = window.entry(path.clone()).or_insert(Instant::now());
        let elapsed = last_seen.elapsed();
        if elapsed >= Duration::from_millis(DEBOUNCE_WINDOW_MS) {
            *last_seen = Instant::now();
            keep = true;
        }
        if matches!(event.kind, NotifyEventKind::Modify(notify::event::ModifyKind::Name(_))) {
            action = FsAction::Rename;
        }
        if !keep && elapsed >= Duration::from_millis(500) {
            *last_seen = Instant::now();
            keep = true;
        }
    }
    if !keep {
        return None;
    }
    *seq += 1;
    let primary = event.paths.first().cloned().unwrap_or_default();
    let ext = primary.extension().map(|s| s.to_string_lossy().to_string());
    let size_hint = std::fs::metadata(&primary).ok().map(|m| m.len());
    Some(FsEvent {
        sequence: *seq,
        action,
        ext,
        size_hint,
    })
}

fn matches_ignore(path: &Path, ignore: &[String]) -> bool {
    ignore.iter().any(|i| path.to_string_lossy().contains(i.as_str()))
}

fn within_root(path: &Path, root: &Path) -> bool {
    let p_norm = path.to_string_lossy().replace('\\', "/");
    let r_norm = root.to_string_lossy().replace('\\', "/");
    p_norm.starts_with(&r_norm)
}

pub fn default_ignore() -> Vec<String> {
    [
        "/.git/",
        "/target/",
        "/node_modules/",
        "/dist/",
        "/build/",
        "/.gradle/",
        "/.idea/",
        "/__pycache__/",
        "/.pytest_cache/",
        "/.mypy_cache/",
        "/venv/",
        "/.venv/",
        "/.runlens/",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_target_dir() {
        let ignore = default_ignore();
        assert!(matches_ignore(Path::new("/x/y/target/foo.rs"), &ignore));
        assert!(!matches_ignore(Path::new("/x/y/src/main.rs"), &ignore));
    }

    #[test]
    fn within_root_respects_unix_and_windows_separators() {
        assert!(within_root(Path::new(r"C:\proj\src"), Path::new(r"C:\proj")));
        assert!(within_root(Path::new("/proj/src/main.rs"), Path::new("/proj")));
        assert!(!within_root(Path::new("/other/x"), Path::new("/proj")));
    }
}
