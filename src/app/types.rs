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
