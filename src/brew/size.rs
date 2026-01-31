use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct SizeEntry {
    pub name: String,
    pub size_kb: u64,
}

pub struct SizesMessage {
    pub result: anyhow::Result<Vec<SizeEntry>>,
}

pub async fn fetch_sizes() -> anyhow::Result<Vec<SizeEntry>> {
    let cellar = fetch_cellar_path().await?;
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

    let output = tokio::process::Command::new("du")
        .arg("-sk")
        .args(&entries)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "du failed".to_string()
        } else {
            stderr
        };
        return Err(anyhow::anyhow!(message));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sizes: Vec<SizeEntry> = stdout
        .lines()
        .filter_map(|line| parse_du_line(line))
        .collect();

    sizes.sort_by(|a, b| b.size_kb.cmp(&a.size_kb));
    Ok(sizes)
}

fn parse_du_line(line: &str) -> Option<SizeEntry> {
    let mut parts = line.split_whitespace();
    let size = parts.next()?.parse::<u64>().ok()?;
    let path = parts.next()?;
    let name = PathBuf::from(path)
        .file_name()
        .map(|os| os.to_string_lossy().to_string())?;
    Some(SizeEntry { name, size_kb: size })
}

async fn fetch_cellar_path() -> anyhow::Result<PathBuf> {
    let output = tokio::process::Command::new("brew")
        .arg("--cellar")
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "brew --cellar failed".to_string()
        } else {
            stderr
        };
        return Err(anyhow::anyhow!(message));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let path = stdout.trim();
    Ok(PathBuf::from(path))
}
