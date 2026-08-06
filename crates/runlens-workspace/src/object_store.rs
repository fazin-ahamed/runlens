use std::path::Path;

pub struct ObjectStore;

impl ObjectStore {
    pub fn open(_path: &Path) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn list_chunks(&self) -> anyhow::Result<Vec<String>> {
        Ok(vec![])
    }

    pub fn read_chunk(&self, _id: &str) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(None)
    }
}
