use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    Search,
    Install,
    Uninstall,
    Upgrade,
    UpgradeAll,
    ServiceStart,
    ServiceStop,
    ServiceRestart,
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
            Self::ServiceStart => "services start",
            Self::ServiceStop => "services stop",
            Self::ServiceRestart => "services restart",
            Self::SelfUpdate => "self-update",
            Self::Cleanup => "cleanup",
            Self::Autoremove => "autoremove",
            Self::BundleDump => "bundle dump",
        }
    }

    pub fn is_package_action(self) -> bool {
        matches!(self, Self::Install | Self::Uninstall | Self::Upgrade)
    }

    pub fn is_service_action(self) -> bool {
        matches!(
            self,
            Self::ServiceStart | Self::ServiceStop | Self::ServiceRestart
        )
    }

    pub fn has_named_target(self) -> bool {
        self.is_package_action() || self.is_service_action()
    }

    pub fn is_activity_command(self) -> bool {
        matches!(
            self,
            Self::Install
                | Self::Uninstall
                | Self::Upgrade
                | Self::UpgradeAll
                | Self::ServiceStart
                | Self::ServiceStop
                | Self::ServiceRestart
                | Self::SelfUpdate
        )
    }

    pub fn refreshes_lists_on_success(self) -> bool {
        matches!(
            self,
            Self::Install
                | Self::Uninstall
                | Self::Upgrade
                | Self::UpgradeAll
                | Self::Cleanup
                | Self::Autoremove
        )
    }

    pub fn refreshes_status_on_success(self) -> bool {
        self.refreshes_lists_on_success() || self.is_service_action()
    }

    pub fn action_title(self) -> &'static str {
        match self {
            Self::Install => "Install",
            Self::Uninstall => "Uninstall",
            Self::Upgrade => "Upgrade",
            Self::ServiceStart => "Start service",
            Self::ServiceStop => "Stop service",
            Self::ServiceRestart => "Restart service",
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
    pub exit_code: Option<i32>,
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
        exit_code: output.status.code(),
    })
}

#[cfg(test)]
mod tests {
    use super::CommandKind;

    #[test]
    fn refreshes_lists_for_expected_commands() {
        assert!(CommandKind::Install.refreshes_lists_on_success());
        assert!(CommandKind::Uninstall.refreshes_lists_on_success());
        assert!(CommandKind::Upgrade.refreshes_lists_on_success());
        assert!(CommandKind::UpgradeAll.refreshes_lists_on_success());
        assert!(CommandKind::Cleanup.refreshes_lists_on_success());
        assert!(CommandKind::Autoremove.refreshes_lists_on_success());
    }

    #[test]
    fn does_not_refresh_lists_for_non_mutating_commands() {
        assert!(!CommandKind::Search.refreshes_lists_on_success());
        assert!(!CommandKind::SelfUpdate.refreshes_lists_on_success());
        assert!(!CommandKind::BundleDump.refreshes_lists_on_success());
        assert!(!CommandKind::ServiceStart.refreshes_lists_on_success());
    }

    #[test]
    fn refreshes_status_for_service_actions() {
        assert!(CommandKind::ServiceStart.refreshes_status_on_success());
        assert!(CommandKind::ServiceStop.refreshes_status_on_success());
        assert!(CommandKind::ServiceRestart.refreshes_status_on_success());
    }
}
