use std::time::Instant;

use crate::brew::{
    CommandKind, CommandResult, DependencyGraph, Details, DetailsLoad, DoctorReport,
    InstalledFormulae, SizeEntry, StatusSnapshot,
};

/// The result of a background request, sent back to the event loop.
pub enum Message {
    Leaves(anyhow::Result<InstalledFormulae>),
    Casks(anyhow::Result<Vec<String>>),
    Details {
        pkg: String,
        load: DetailsLoad,
        result: anyhow::Result<Details>,
    },
    Sizes(anyhow::Result<Vec<SizeEntry>>),
    Command {
        kind: CommandKind,
        result: anyhow::Result<CommandResult>,
    },
    Status(anyhow::Result<StatusSnapshot>),
    Graph(anyhow::Result<DependencyGraph>),
    Doctor(anyhow::Result<DoctorReport>),
}

/// One kind of background request: whether it is in flight, and when it last
/// completed successfully.
#[derive(Clone, Copy, Default)]
pub struct Job {
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
}

impl Job {
    pub fn is_running(&self) -> bool {
        self.started_at.is_some()
    }

    /// Marks the job as started. Returns false, changing nothing, if it
    /// already was.
    pub fn start(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        self.started_at = Some(Instant::now());
        true
    }

    pub fn finish(&mut self, succeeded: bool) {
        self.started_at = None;
        if succeeded {
            self.finished_at = Some(Instant::now());
        }
    }

    pub fn elapsed_secs(&self) -> u64 {
        self.started_at
            .map_or(0, |started| started.elapsed().as_secs())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    SearchLeaves,
    PackageSearch,
    PackageResults,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PackageAction {
    Install,
    Uninstall,
    Upgrade,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

impl PackageKind {
    pub fn noun(self) -> &'static str {
        match self {
            Self::Formula => "formula",
            Self::Cask => "cask",
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Self::Formula => "formulae",
            Self::Cask => "casks",
        }
    }
}

/// How the installed formula list is ordered.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SortMode {
    #[default]
    Name,
    /// Largest first, from the size scan; unsized entries trail by name.
    Size,
    /// Most recently installed first, from install receipts.
    Recent,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Size,
            Self::Size => Self::Recent,
            Self::Recent => Self::Name,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Size => "size",
            Self::Recent => "recent",
        }
    }
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

/// A destructive action armed by its first key press, which runs on the
/// second press of the same key and is dropped by anything else.
#[derive(Clone, PartialEq)]
pub enum Confirmation {
    Package(PendingPackageAction),
    Service(PendingServiceAction),
    UpgradeAllOutdated,
    Autoremove,
    SelfUpdate,
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

impl StatusTab {
    /// In display order.
    pub const ALL: [Self; 5] = [
        Self::Activity,
        Self::Issues,
        Self::Outdated,
        Self::Services,
        Self::History,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Activity => "Activity",
            Self::Issues => "Issues",
            Self::Outdated => "Outdated",
            Self::Services => "Services",
            Self::History => "History",
        }
    }

    pub fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        Self::ALL[(self as usize + Self::ALL.len() - 1) % Self::ALL.len()]
    }
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

#[cfg(test)]
mod tests {
    use super::StatusTab;

    #[test]
    fn status_tabs_cycle_in_display_order() {
        assert_eq!(StatusTab::Activity.next(), StatusTab::Issues);
        assert_eq!(StatusTab::History.next(), StatusTab::Activity);
        assert_eq!(StatusTab::Activity.prev(), StatusTab::History);
        for tab in StatusTab::ALL {
            assert_eq!(tab.next().prev(), tab);
        }
    }
}
