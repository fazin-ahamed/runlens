//! Read-only git fingerprint snapshot.
//!
//! Shell-out to `git` rather than depending on libgit2. Output is a small
//! JSON payload that goes into the session's `git.snapshot` event.
//!
//! Coverage:
//!   - current HEAD (short/full)
//!   - current branch (or detached)
//!   - clean/dirty detection
//!   - modified filenames (path-masked through [`mask_absolute_path`])
//!   - lockfile hashes (Cargo.lock, package-lock.json, pnpm-lock.yaml,
//!     poetry.lock, yarn.lock) — fairly common, bluesky-stable technique to
//!     detect drifted dependency state without diffing the lockfile body.

use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use blake3::Hasher;
use runlens_core::privacy::mask_absolute_path;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GitFingerprint {
    pub present: bool,
    pub head: Option<String>,
    pub head_full: Option<String>,
    pub branch: Option<String>,
    pub dirty: bool,
    pub modified: Vec<String>,
    pub lockfile_hashes: indexmap::IndexMap<String, String>,
}

/// Try to capture a fingerprint; returns [`Err`] if git is missing or the
/// path is not a repo, but callers typically treat Err as `present=false`.
pub async fn capture_git_fingerprint(root: &Path) -> Result<GitFingerprint> {
    let path = root.to_path_buf();
    tokio::task::spawn_blocking(move || capture_blocking(&path)).await?
}

fn capture_blocking(root: &Path) -> Result<GitFingerprint> {
    let git = |args: &[&str]| -> Result<std::process::Output> {
        let output = Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .with_context(|| format!("git {:?}", args))?;
        if !output.status.success() {
            return Err(anyhow!(
                "git {:?} failed with status {}",
                args,
                output.status
            ));
        }
        Ok(output)
    };

    let present = git(&["rev-parse", "--git-dir"]).is_ok();
    if !present {
        return Ok(GitFingerprint {
            present: false,
            head: None,
            head_full: None,
            branch: None,
            dirty: false,
            modified: vec![],
            lockfile_hashes: indexmap::IndexMap::new(),
        });
    }

    let head = git(&["rev-parse", "--short=12", "HEAD"])
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let head_full = git(&["rev-parse", "HEAD"])
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let branch = git(&["branch", "--show-current"])
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let porcelain = git(&["status", "--porcelain", "--untracked-files=no"])
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    let dirty = !porcelain.is_empty();
    let modified: Vec<String> = porcelain
        .lines()
        .map(|l| l.get(3..).unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .map(|s| {
            mask_absolute_path(&s, &root.to_string_lossy(), &whoami())
        })
        .take(100)
        .collect();

    let mut lockfile_hashes = indexmap::IndexMap::new();
    for lock in &[
        "Cargo.lock",
        "package-lock.json",
        "pnpm-lock.yaml",
        "poetry.lock",
        "yarn.lock",
        "composer.lock",
        "Gemfile.lock",
        "go.sum",
    ] {
        let path = root.join(lock);
        if path.is_file() {
            let hash = hash_file(&path)
                .with_context(|| format!("hash lockfile {:?}", path))?;
            let masked = mask_absolute_path(
                &path.to_string_lossy(),
                &root.to_string_lossy(),
                &whoami(),
            );
            lockfile_hashes.insert(masked, hash);
        }
    }

    Ok(GitFingerprint {
        present: true,
        head,
        head_full,
        branch,
        dirty,
        modified,
        lockfile_hashes,
    })
}

fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("read {:?}", path))?;
    let mut h = Hasher::new();
    h.update(&bytes);
    Ok(format!("blake3:{}", hex_short(&h.finalize().as_bytes()[..16])))
}

fn hex_short(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_hashes_distinguish_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("Cargo.lock");
        let b = dir.path().join("other.lock");
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        let h1 = hash_file(&a).unwrap();
        let h2 = hash_file(&b).unwrap();
        assert_ne!(h1, h2);
        assert!(h1.starts_with("blake3:"));
    }
}
