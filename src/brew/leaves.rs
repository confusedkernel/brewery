use super::process::{ensure_success, nonempty_lines, run_brew};

pub struct LeavesMessage {
    pub result: anyhow::Result<Vec<String>>,
}

pub async fn fetch_leaves() -> anyhow::Result<Vec<String>> {
    let output = run_brew(&["leaves"]).await?;
    ensure_success(&output, "brew leaves failed")?;
    Ok(nonempty_lines(&output.stdout))
}
