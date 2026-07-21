fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let port: Option<u16> = if args.len() > 1 {
        args[1].parse().ok()
    } else {
        None
    };
    let cwd = std::env::current_dir().unwrap_or_default();
    let db_path = cwd.join(".runlens").join("runlens.sqlite");
    let repo = runlens_storage::Repository::open(&db_path)?;
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(async move {
        match port {
            None => runlens_mcp::stdio(repo).await,
            Some(p) => runlens_mcp::http(repo, p).await,
        }
    })
}
