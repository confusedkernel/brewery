use std::path::PathBuf;
use std::process::Output;

/// The Cellar root, where per-formula keg directories and their install
/// receipts live.
pub(super) async fn cellar_path() -> anyhow::Result<PathBuf> {
    let output = run_brew(&["--cellar"]).await?;
    ensure_success(&output, "brew --cellar failed")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(PathBuf::from(stdout.trim()))
}

pub(super) async fn run_brew(args: &[&str]) -> anyhow::Result<Output> {
    Ok(tokio::process::Command::new("brew")
        .args(args)
        .output()
        .await?)
}

pub(super) fn ensure_success(output: &Output, fallback: &str) -> anyhow::Result<()> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let message = if stderr.is_empty() {
        fallback.to_string()
    } else {
        stderr
    };
    Err(anyhow::anyhow!(message))
}

pub(super) fn nonempty_lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}
