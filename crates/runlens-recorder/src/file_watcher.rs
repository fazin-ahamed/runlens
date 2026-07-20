//! Notifications via [`notify`] with manual debouncing.
//!
//! Runlens is typically invoked inside a watch loop (e.g. `runlens record
//! --watch`), so we want:
//!   - Underlying watch events coalesced across roughly 50 ms.
//!   - Recursive watching of project directories with selective ignores.
//!   - Bundle-friendly serialised payloads so events can join a session
//!     without revealing absolute paths.

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

/// Build a watcher for `root` and yield a `mpsc::Receiver<FSEvent>` with
/// debounced events. The watcher is owned by the returned [`FsWatcher`]
/// so dropping it will stop the watch loop.
pub struct FsWatcher {
    _watcher: RecommendedWatcher,
    pub rx: mpsc::Receiver<FsEvent>,
    _seq: u64,
}

impl FsWatcher {
    pub fn start(root: &Path, ignore: &[String]) -> Result<Self> {
        let (raw_tx, raw_rx) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = raw_tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_millis(200)),
        )
        .context("construct notify watcher")?;
        watcher
            .watch(root, RecursiveMode::Recursive)
            .with_context(|| format!("watch {:?}", root))?;

        let (tx, rx) = mpsc::channel::<FsEvent>();
        let mut seq: u64 = 0;
        let root_buf = root.to_path_buf();
        let ignore = ignore.to_vec();
        std::thread::spawn(move || loop {
            match raw_rx.recv() {
                Ok(Ok(event)) => {
                    let _ = Instant::now();
                    let debounced =
                        debounce(&mut Debounced::default(), &event, &root_buf, &ignore, &mut seq);
                    if let Some(out) = debounced {
                        if tx.send(out).is_err() {
                            break;
                        }
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
        let entry = window.entry(path.clone()).or_insert(Instant::now());
        let elapsed = entry.elapsed();
        if elapsed >= Duration::from_millis(DEBOUNCE_WINDOW_MS) {
            *entry = Instant::now();
            keep = true;
        }
        // If remove supersedes create on the same path we collapse: remove-with-prev-create is a rename hint.
        if matches!(event.kind, NotifyEventKind::Modify(notify::event::ModifyKind::Name(_))) {
            action = FsAction::Rename;
        }
        if !keep {
            // We will still allow updates through occasionally to keep surface fluids.
            if elapsed >= Duration::from_millis(500) {
                *entry = Instant::now();
                keep = true;
            }
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

/// Default ignores. Conservative — caller may override.
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
