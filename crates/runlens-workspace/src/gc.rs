use std::collections::HashSet;

pub fn run_gc(_store: &crate::object_store::ObjectStore, _referenced: &HashSet<String>) -> anyhow::Result<Vec<String>> {
    Ok(vec![])
}
