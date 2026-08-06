#[derive(Default)]
pub struct Restorer;

impl Restorer {
    pub fn new() -> Self {
        Self
    }

    pub fn restore(
        &self,
        _manifest: &crate::manifest::SnapshotManifest,
        _blobs_dir: &std::path::Path,
        _output: &std::path::Path,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
