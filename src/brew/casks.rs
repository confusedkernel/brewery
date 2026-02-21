use super::process::{ensure_success, nonempty_lines, run_brew};

pub struct CasksMessage {
    pub result: anyhow::Result<Vec<String>>,
}

pub async fn fetch_casks() -> anyhow::Result<Vec<String>> {
    let output = run_brew(&["list", "--cask"]).await?;
    ensure_success(&output, "brew list --cask failed")?;
    Ok(nonempty_lines(&output.stdout))
}
