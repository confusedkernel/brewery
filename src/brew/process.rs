use std::process::Output;

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
