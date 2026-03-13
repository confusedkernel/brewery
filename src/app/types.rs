use std::time::Instant;

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    SearchLeaves,
    PackageSearch,
    PackageResults,
}

#[derive(Clone, Copy, PartialEq)]
pub enum IconMode {
    Auto,
    Nerd,
    Ascii,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PackageAction {
    Install,
    Uninstall,
    Upgrade,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ServiceKindFilter {
    #[default]
    All,
    Formula,
    Cask,
}

impl ServiceKindFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Formula,
            Self::Formula => Self::Cask,
            Self::Cask => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Formula => "formula",
            Self::Cask => "cask",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PackageKind {
    Formula,
    Cask,
}

#[derive(Clone, PartialEq)]
pub struct PendingPackageAction {
    pub action: PackageAction,
    pub kind: PackageKind,
    pub pkg: String,
}

#[derive(Clone, PartialEq)]
pub struct PendingServiceAction {
    pub action: ServiceAction,
    pub service: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    Details,
    PackageResults,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FocusedPanel {
    Leaves,
    Sizes,
    Status,
    Details,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum StatusTab {
    #[default]
    Activity,
    Issues,
    Outdated,
    Services,
    History,
}

#[derive(Clone)]
pub struct CommandHistoryEntry {
    pub kind: String,
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub finished_at: Instant,
    pub duration_secs: u64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ToastLevel {
    Success,
    Error,
}

#[derive(Clone)]
pub struct Toast {
    pub level: ToastLevel,
    pub message: String,
    pub created_at: Instant,
}
