pub fn fetch_leaves() -> anyhow::Result<Vec<String>> {
    let output = std::process::Command::new("brew").arg("leaves").output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "brew leaves failed".to_string()
        } else {
            stderr
        };
        return Err(anyhow::anyhow!(message));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let leaves = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(leaves)
}
