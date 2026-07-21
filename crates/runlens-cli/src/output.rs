//! Output helpers: table, json, markdown.

pub fn render_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Render a small text table from rows of strings.
pub fn render_table(headers: &[&str], rows: Vec<Vec<String>>) {
    if rows.is_empty() {
        return;
    }
    let widths = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let max = rows
                .iter()
                .map(|r| r.get(i).map(|s| s.len()).unwrap_or(0))
                .max()
                .unwrap_or(0);
            std::cmp::max(max, h.len())
        })
        .collect::<Vec<_>>();
    let mut line = String::new();
    for (i, h) in headers.iter().enumerate() {
        line.push_str(&format!("{:<width$}  ", h, width = widths[i]));
    }
    println!("{line}");
    println!("{}", "-".repeat(line.len()));
    for row in rows {
        let mut out = String::new();
        for (i, cell) in row.iter().enumerate() {
            out.push_str(&format!("{:<width$}  ", cell, width = widths[i]));
        }
        println!("{out}");
    }
}
