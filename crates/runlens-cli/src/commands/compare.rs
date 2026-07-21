pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    baseline: &str,
    candidate: &str,
    json: bool,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let a = repo.list_events(baseline).await?;
    let b = repo.list_events(candidate).await?;
    let cmp = runlens_core::compare::compare_sessions(&a, &b);
    if json {
        crate::output::render_json(&cmp)?;
        return Ok(());
    }
    println!(
        "Comparison: {} -> {}",
        baseline,
        candidate
    );
    println!(
        "  events: baseline={}, candidate={}",
        cmp.baseline_event_count, cmp.candidate_event_count
    );
    for (i, d) in cmp.divergences.iter().enumerate() {
        println!(
            "  {}. [{:?}] {}\n     {}",
            i + 1,
            d.severity,
            d.title,
            d.summary
        );
    }
    if cmp.divergences.is_empty() {
        println!("  no divergences surfaced");
    }
    Ok(())
}
