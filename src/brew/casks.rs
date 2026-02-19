pub struct CasksMessage {
    pub result: anyhow::Result<Vec<String>>,
}

pub async fn fetch_casks() -> anyhow::Result<Vec<String>> {
    let output = tokio::process::Command::new("brew")
        .args(["list", "--cask"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "brew list --cask failed".to_string()
        } else {
            stderr
        };
        return Err(anyhow::anyhow!(message));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let casks = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(casks)
}
