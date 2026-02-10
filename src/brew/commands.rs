use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    Search,
    Install,
    Uninstall,
    Upgrade,
    UpgradeAll,
    SelfUpdate,
    Cleanup,
    Autoremove,
    BundleDump,
}

impl CommandKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Install => "install",
            Self::Uninstall => "uninstall",
            Self::Upgrade => "upgrade",
            Self::UpgradeAll => "upgrade-all",
            Self::SelfUpdate => "self-update",
            Self::Cleanup => "cleanup",
            Self::Autoremove => "autoremove",
            Self::BundleDump => "bundle dump",
        }
    }

    pub fn is_package_action(self) -> bool {
        matches!(self, Self::Install | Self::Uninstall | Self::Upgrade)
    }

    pub fn is_activity_command(self) -> bool {
        matches!(
            self,
            Self::Install | Self::Uninstall | Self::Upgrade | Self::UpgradeAll | Self::SelfUpdate
        )
    }

    pub fn refreshes_lists_on_success(self) -> bool {
        matches!(
            self,
            Self::Install | Self::Uninstall | Self::Upgrade | Self::UpgradeAll
        )
    }

    pub fn action_title(self) -> &'static str {
        match self {
            Self::Install => "Install",
            Self::Uninstall => "Uninstall",
            Self::Upgrade => "Upgrade",
            _ => "Action",
        }
    }
}

impl fmt::Display for CommandKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Clone, Debug)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub struct CommandMessage {
    pub kind: CommandKind,
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
