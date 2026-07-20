//! Content-addressed artifact stores. Stored on disk under a content hash.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Filesystem artifact store. Writes are atomic and verified by hash.
pub struct DiskArtifacts {
    root: PathBuf,
}

impl DiskArtifacts {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root).context("creating artifact root")?;
        Ok(Self { root })
    }

    /// Atomically write a payload under its content hash. Idempotent.
    pub fn put(&self, content_hash: &str, payload: &[u8]) -> Result<PathBuf> {
        let path = self.resolve_path(content_hash);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.exists() {
            // Verify existing.
            let existing = std::fs::read(&path)?;
            if existing != payload {
                // Hash collision or external tamper; refuse.
                anyhow::bail!("artifact exists but content differs: {content_hash}");
            }
            return Ok(path);
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, payload).context("writing tmp artifact")?;
        std::fs::rename(&tmp, &path).context("renaming artifact")?;
        Ok(path)
    }

    pub fn read(&self, content_hash: &str) -> Result<Vec<u8>> {
        let p = self.resolve_path(content_hash);
        let bytes = std::fs::read(&p).with_context(|| format!("reading artifact {content_hash}"))?;
        let actual = blake3::hash(&bytes).to_hex().to_string();
        if actual != content_hash {
            anyhow::bail!("artifact {content_hash} hash mismatch on reread");
        }
        Ok(bytes)
    }

    pub fn exists(&self, content_hash: &str) -> bool {
        self.resolve_path(content_hash).exists()
    }

    pub fn total_size(&self) -> Result<u64> {
        let mut total = 0u64;
        for entry in walkdir::WalkDir::new(&self.root).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
        Ok(total)
    }

    fn resolve_path(&self, hash: &str) -> PathBuf {
        // Two-level fan-out: ab/cd/<hash>.bin
        let prefix1 = &hash[..2];
        let prefix2 = &hash[2..4];
        self.root.join(prefix1).join(prefix2).join(format!("{hash}.bin"))
    }
}

/// Lightweight DB-backed artifact index.
pub fn register(conn: &Connection, content_hash: &str, size: u64, media: &str, origin: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO artifacts(content_hash, size_bytes, media_kind, origin) VALUES (?1, ?2, ?3, ?4)",
        params![content_hash, size as i64, media, origin],
    )?;
    Ok(())
}

pub fn link_event(conn: &Connection, event_id: &str, artifact: &str, role: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO event_artifacts(event_id, artifact_hash, role) VALUES (?1, ?2, ?3)",
        params![event_id, artifact, role],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn put_and_read_round_trips_with_hash_verification() {
        let dir = TempDir::new().unwrap();
        let store = DiskArtifacts::open(dir.path()).unwrap();
        let payload = b"hello, world";
        let hash = blake3::hash(payload).to_hex().to_string();
        let path = store.put(&hash, payload).unwrap();
        assert!(path.exists());
        let read = store.read(&hash).unwrap();
        assert_eq!(payload, &read[..]);
    }

    #[test]
    fn put_is_idempotent_for_same_content() {
        let dir = TempDir::new().unwrap();
        let store = DiskArtifacts::open(dir.path()).unwrap();
        let payload = vec![7u8; 1024];
        let hash = blake3::hash(&payload).to_hex().to_string();
        let p1 = store.put(&hash, &payload).unwrap();
        let p2 = store.put(&hash, &payload).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn put_rejects_different_content_under_same_hash() {
        let dir = TempDir::new().unwrap();
        let store = DiskArtifacts::open(dir.path()).unwrap();
        let payload = b"abc";
        let hash = blake3::hash(payload).to_hex().to_string();
        store.put(&hash, payload).unwrap();
        let res = store.put(&hash, b"different");
        assert!(res.is_err());
    }

    #[test]
    fn read_detects_tampering() {
        let dir = TempDir::new().unwrap();
        let store = DiskArtifacts::open(dir.path()).unwrap();
        let payload = b"original";
        let hash = blake3::hash(payload).to_hex().to_string();
        store.put(&hash, payload).unwrap();
        let p = store.resolve_path(&hash);
        std::fs::write(&p, b"tampered").unwrap();
        assert!(store.read(&hash).is_err());
    }
}
