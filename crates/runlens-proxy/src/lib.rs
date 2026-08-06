pub mod proxy;

pub mod tls {
    use std::path::Path;

    pub struct CaStore;

    impl CaStore {
        pub fn load_or_generate(_path: &Path) -> anyhow::Result<Self> {
            Ok(Self)
        }

        pub fn install(&self) -> anyhow::Result<()> {
            Ok(())
        }

        pub fn remove(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }
}
