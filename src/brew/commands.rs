#[derive(Clone, Debug)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub struct CommandMessage {
    pub label: String,
    pub result: anyhow::Result<CommandResult>,
}

pub async fn run_brew_command(args: &[&str]) -> anyhow::Result<CommandResult> {
    run_command("brew", args).await
}

pub async fn run_command(binary: &str, args: &[&str]) -> anyhow::Result<CommandResult> {
    let output = tokio::process::Command::new(binary)
        .args(args)
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(CommandResult {
        stdout,
        stderr,
        success: output.status.success(),
    })
}
