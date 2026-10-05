use std::path::PathBuf;

use super::process::{cellar_path, ensure_success, run_background};

#[derive(Clone, Debug)]
pub struct SizeEntry {
    pub name: String,
    pub size_kb: u64,
}

pub async fn fetch_sizes() -> anyhow::Result<Vec<SizeEntry>> {
    let cellar = cellar_path().await?;
    let mut entries = Vec::new();

    for dir in std::fs::read_dir(&cellar)? {
        let dir = dir?;
        if dir.file_type()?.is_dir() {
            entries.push(dir.path());
        }
    }

    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let mut args = vec![PathBuf::from("-sk")];
    args.extend(entries);
    let output = run_background("du", &args).await?;

    ensure_success(&output, "du failed")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sizes: Vec<SizeEntry> = stdout.lines().filter_map(parse_du_line).collect();

    sizes.sort_by_key(|entry| std::cmp::Reverse(entry.size_kb));
    Ok(sizes)
}

fn parse_du_line(line: &str) -> Option<SizeEntry> {
    let mut parts = line.split_whitespace();
    let size = parts.next()?.parse::<u64>().ok()?;
    let path = parts.next()?;
    let name = PathBuf::from(path)
        .file_name()
        .map(|os| os.to_string_lossy().to_string())?;
    Some(SizeEntry {
        name,
        size_kb: size,
    })
}
