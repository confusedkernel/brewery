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

#[derive(Clone, PartialEq)]
pub enum PackageAction {
    Install,
    Uninstall,
    Upgrade,
}

#[derive(Clone, PartialEq)]
pub struct PendingPackageAction {
    pub action: PackageAction,
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
pub enum HealthTab {
    #[default]
    Activity,
    Issues,
    Outdated,
}
