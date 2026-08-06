pub mod proxy;

pub mod tls {
    use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
    use std::fs;
    use std::path::{Path, PathBuf};

    const CERT_FILE: &str = "runlens-ca.crt";
    const KEY_FILE: &str = "runlens-ca.key";

    /// Load an existing RunLens CA, or generate a new self-signed CA and
    /// persist it as PEM under `dir` so the key survives restarts.
    pub struct CaStore {
        dir: PathBuf,
        cert_pem: String,
        key_pem: String,
    }

    impl CaStore {
        pub fn load_or_generate(dir: &Path) -> anyhow::Result<Self> {
            let dir = dir.to_path_buf();
            let cert_path = dir.join(CERT_FILE);
            let key_path = dir.join(KEY_FILE);

            if cert_path.exists() && key_path.exists() {
                let cert_pem = fs::read_to_string(&cert_path)?;
                let key_pem = fs::read_to_string(&key_path)?;
                return Ok(Self { dir, cert_pem, key_pem });
            }

            fs::create_dir_all(&dir)?;

            let mut params = CertificateParams::new(vec!["RunLens CA".to_string()])?;
            params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            params
                .distinguished_name
                .push(rcgen::DnType::CommonName, "RunLens Local CA");

            let key_pair = KeyPair::generate()?;
            let cert = params.self_signed(&key_pair)?;
            let cert_pem = cert.pem();
            let key_pem = key_pair.serialize_pem();

            fs::write(&cert_path, &cert_pem)?;
            fs::write(&key_path, &key_pem)?;

            Ok(Self { dir, cert_pem, key_pem })
        }

        pub fn cert_path(&self) -> PathBuf {
            self.dir.join(CERT_FILE)
        }

        pub fn cert_pem(&self) -> &str {
            &self.cert_pem
        }

        pub fn key_pem(&self) -> &str {
            &self.key_pem
        }

        /// Install the CA into the platform trust store.
        /// Returns Ok(()) even when no trust store is writable so the proxy
        /// can still run in explicit trust mode.
        pub fn install(&self) -> Result<(), anyhow::Error> {
            let cert_path = self.cert_path();
            install_to_store(&cert_path)
        }

        /// Remove the CA from the platform trust store.
        pub fn remove(&self) -> Result<(), anyhow::Error> {
            let cert_path = self.cert_path();
            remove_from_store(&cert_path)
        }
    }

    #[cfg(target_os = "macos")]
    fn install_to_store(cert_path: &Path) -> anyhow::Result<()> {
        let status = std::process::Command::new("security")
            .args([
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                "~/Library/Keychains/login.keychain-db",
            ])
            .arg(cert_path)
            .status()?;
        if !status.success() {
            anyhow::bail!("`security add-trusted-cert` exited with status {status}");
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn remove_from_store(cert_path: &Path) -> anyhow::Result<()> {
        let status = std::process::Command::new("security")
            .arg("delete-certificate")
            .arg("-c")
            .arg("RunLens Local CA")
            .status()?;
        if !status.success() {
            anyhow::bail!("`security delete-certificate` exited with status {status}");
        }
        let _ = cert_path;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn install_to_store(cert_path: &Path) -> anyhow::Result<()> {
        let dest = Path::new("/usr/local/share/ca-certificates/runlens-ca.crt");
        let status = std::process::Command::new("sudo")
            .args(["cp", cert_path.to_str().unwrap_or(""), dest.to_str().unwrap_or("")])
            .status()?;
        if !status.success() {
            anyhow::bail!("`sudo cp` exited with status {status}");
        }
        let _ = std::process::Command::new("sudo")
            .arg("update-ca-certificates")
            .status();
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn remove_from_store(_cert_path: &Path) -> anyhow::Result<()> {
        let dest = Path::new("/usr/local/share/ca-certificates/runlens-ca.crt");
        if dest.exists() {
            std::process::Command::new("sudo")
                .args(["rm", "-f", dest.to_str().unwrap_or("")])
                .status()?;
            let _ = std::process::Command::new("sudo")
                .arg("update-ca-certificates")
                .status();
        }
        Ok(())
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "linux")))]
    fn install_to_store(_cert_path: &Path) -> anyhow::Result<()> {
        // Windows: place the PEM into the current-user root store.
        let cert = _cert_path.to_str().unwrap_or("");
        let pwsh = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "$c=New-Object System.Security.Cryptography.X509Certificates.X509Certificate2('{cert}'); \
                     $s=New-Object System.Security.Cryptography.X509Certificates.X509Store 'Root','CurrentUser'; \
                     $s.Open([System.Security.Cryptography.X509Certificates.OpenFlags]'ReadWrite'); \
                     $s.Add($c); $s.Close()"
                ),
            ])
            .output()?;
        if !pwsh.status.success() {
            anyhow::bail!(
                "powershell cert install failed: {}",
                String::from_utf8_lossy(&pwsh.stderr)
            );
        }
        Ok(())
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "linux")))]
    fn remove_from_store(_cert_path: &Path) -> anyhow::Result<()> {
        let name = "RunLens Local CA";
        let pwsh = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "$s=New-Object System.Security.Cryptography.X509Certificates.X509Store 'Root','CurrentUser'; \
                     $s.Open('ReadWrite'); $s.Certificates | Where-Object {{ $_.Subject -like '*{name}*' }} | \
                     ForEach-Object {{ $s.Remove($_) }}; $s.Close()"
                ),
            ])
            .output()?;
        let _ = pwsh;
        Ok(())
    }
}
