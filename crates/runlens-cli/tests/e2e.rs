use std::path::PathBuf;
use std::process::Command;

fn runlens_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_runlens"))
}

fn temp_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut git_dir = dir.path().to_path_buf();
    git_dir.push(".git");
    std::fs::create_dir_all(&git_dir).unwrap();
    dir
}

#[test]
fn e2e_init_creates_store() {
    let dir = temp_project();
    let bin = runlens_bin();
    let out = Command::new(&bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("runlens init");
    assert!(
        out.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let runlens_dir = dir.path().join(".runlens");
    assert!(runlens_dir.exists(), ".runlens should exist");
}

#[test]
fn e2e_help_succeeds() {
    let bin = runlens_bin();
    let out = Command::new(&bin).arg("--help").output().expect("runlens --help");
    assert!(
        out.status.success(),
        "help failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("runlens"), "help should mention runlens");
}

#[test]
fn e2e_list_empty() {
    let dir = temp_project();
    let bin = runlens_bin();
    Command::new(&bin).arg("init").current_dir(dir.path()).output().unwrap();
    let out = Command::new(&bin)
        .arg("list")
        .current_dir(dir.path())
        .output()
        .expect("runlens list");
    assert!(
        out.status.success(),
        "list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
