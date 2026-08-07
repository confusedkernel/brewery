use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{
    App, FocusedPanel, InputMode, PackageAction, PackageKind, PendingPackageAction,
    PendingServiceAction, ServiceAction, StatusTab, ViewMode,
};
use crate::brew::{CommandKind, DetailsLoad};
use crate::runtime::messages::{RuntimeChannels, handle_focus_backtab};
use crate::ui::keymap;

/// Whether a key press was recognized, and whether it ends the session.
///
/// `Unhandled` is what lets the keymap drift test tell "this binding does
/// nothing here" apart from "this binding does not exist".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KeyOutcome {
    Handled,
    Unhandled,
    Quit,
}

enum HelpPopupAction {
    NotHandled,
    Handled,
    Execute(KeyEvent),
}

/// Keys that work in every input mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GlobalAction {
    ToggleHelp,
    ToggleIcons,
}

/// Everything reachable from a normal-mode key press.
///
/// Recognition is deliberately context-free: a key maps to an action no matter
/// what is focused, and the action's handler decides whether it applies. That
/// keeps [`normal_action_for`] pure and exhaustively testable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NormalAction {
    Quit,
    Cancel,
    RefreshLists,
    CycleTheme,
    ToggleMouse,
    LoadSizes,
    StatusCheck,
    ToggleView,
    SearchInstalled,
    FindPackages,
    ToggleOutdatedFilter,
    ToggleInstalledKind,
    ToggleLeavesScope,
    Package(PackageAction),
    UpgradeSelectedOrOutdated,
    Service(ServiceAction),
    ServiceInfo,
    ToggleServicesFailedFilter,
    ToggleServicesAutostartFilter,
    CycleServicesKindFilter,
    SelfUpdate,
    Cleanup,
    Autoremove,
    BundleDump,
    LoadDetails,
    LoadDepsUses,
    FocusNext,
    FocusPrev,
    ScrollUp,
    ScrollDown,
    PrevStatusTab,
    NextStatusTab,
}

pub fn handle_key_event(
    app: &mut App,
    key: KeyEvent,
    channels: &RuntimeChannels,
    help_max_offset: usize,
) -> Option<anyhow::Result<()>> {
    app.needs_redraw = true;

    if app.input_mode == InputMode::Normal
        && let Some(action) = global_action_for(key)
    {
        run_global_action(app, action);
        return None;
    }

    match handle_help_popup_input(app, key, help_max_offset) {
        HelpPopupAction::NotHandled => {}
        HelpPopupAction::Handled => return None,
        HelpPopupAction::Execute(command_key) => {
            app.show_help_popup = false;
            app.help_scroll_offset = 0;
            return handle_key_event(app, command_key, channels, help_max_offset);
        }
    }

    let outcome = match app.input_mode {
        InputMode::Normal => handle_normal_mode_key(app, key, channels),
        InputMode::SearchLeaves => handle_search_leaves_mode_key(app, key),
        InputMode::PackageSearch => handle_package_search_mode_key(app, key, channels),
        InputMode::PackageResults => handle_package_results_mode_key(app, key, channels),
    };

    match outcome {
        KeyOutcome::Quit => Some(Ok(())),
        KeyOutcome::Handled | KeyOutcome::Unhandled => None,
    }
}

fn global_action_for(key: KeyEvent) -> Option<GlobalAction> {
    match key.code {
        KeyCode::Char('?') => Some(GlobalAction::ToggleHelp),
        KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::ALT) => {
            Some(GlobalAction::ToggleIcons)
        }
        _ => None,
    }
}

fn run_global_action(app: &mut App, action: GlobalAction) {
    match action {
        GlobalAction::ToggleHelp => app.toggle_help(),
        GlobalAction::ToggleIcons => app.toggle_icons(),
    }
}

fn handle_help_popup_input(
    app: &mut App,
    key: KeyEvent,
    help_max_offset: usize,
) -> HelpPopupAction {
    if !app.show_help_popup {
        return HelpPopupAction::NotHandled;
    }

    let command_count = keymap::command_count();
    if command_count == 0 {
        app.help_selected_command = 0;
    } else {
        app.help_selected_command = app.help_selected_command.min(command_count - 1);
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('?') => {
            app.show_help_popup = false;
            app.help_scroll_offset = 0;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if command_count > 0 {
                app.help_selected_command = (app.help_selected_command + 1).min(command_count - 1);
                sync_help_scroll_to_selection(app, help_max_offset);
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if command_count > 0 {
                app.help_selected_command = app.help_selected_command.saturating_sub(1);
                sync_help_scroll_to_selection(app, help_max_offset);
            }
        }
        KeyCode::Enter => {
            if let Some(command) = keymap::command_at(app.help_selected_command) {
                return HelpPopupAction::Execute(command.key_event());
            }
        }
        _ => {}
    }

    HelpPopupAction::Handled
}

fn sync_help_scroll_to_selection(app: &mut App, help_max_offset: usize) {
    let Some(selected_line) = keymap::command_line(app.help_selected_command) else {
        app.help_scroll_offset = 0;
        return;
    };

    let total_lines = keymap::line_count();
    let visible_height = total_lines.saturating_sub(help_max_offset).max(1);

    if selected_line < app.help_scroll_offset {
        app.help_scroll_offset = selected_line;
    } else {
        let visible_end = app.help_scroll_offset + visible_height;
        if selected_line >= visible_end {
            app.help_scroll_offset = selected_line + 1 - visible_height;
        }
    }

    app.help_scroll_offset = app.help_scroll_offset.min(help_max_offset);
}

fn normal_action_for(key: KeyEvent) -> Option<NormalAction> {
    let action = match key.code {
        KeyCode::Char('q') => NormalAction::Quit,
        KeyCode::Esc => NormalAction::Cancel,
        KeyCode::Char('r') => NormalAction::RefreshLists,
        KeyCode::Char('t') => NormalAction::CycleTheme,
        KeyCode::Char('m') => NormalAction::ToggleMouse,
        KeyCode::Char('s') => NormalAction::LoadSizes,
        KeyCode::Char('h') => NormalAction::StatusCheck,
        KeyCode::Char('v') => NormalAction::ToggleView,
        KeyCode::Char('/') => NormalAction::SearchInstalled,
        KeyCode::Char('f') => NormalAction::FindPackages,
        KeyCode::Char('o') => NormalAction::ToggleOutdatedFilter,
        KeyCode::Char('C') => NormalAction::ToggleInstalledKind,
        KeyCode::Char('L') => NormalAction::ToggleLeavesScope,
        KeyCode::Char('i') => NormalAction::Package(PackageAction::Install),
        KeyCode::Char('u') => NormalAction::Package(PackageAction::Uninstall),
        KeyCode::Char('U') => NormalAction::UpgradeSelectedOrOutdated,
        KeyCode::Char('S') => NormalAction::Service(ServiceAction::Start),
        KeyCode::Char('X') => NormalAction::Service(ServiceAction::Stop),
        KeyCode::Char('R') => NormalAction::Service(ServiceAction::Restart),
        KeyCode::Char('I') => NormalAction::ServiceInfo,
        KeyCode::Char('F') => NormalAction::ToggleServicesFailedFilter,
        KeyCode::Char('A') => NormalAction::ToggleServicesAutostartFilter,
        KeyCode::Char('K') => NormalAction::CycleServicesKindFilter,
        KeyCode::Char('P') => NormalAction::SelfUpdate,
        KeyCode::Char('c') => NormalAction::Cleanup,
        KeyCode::Char('a') => NormalAction::Autoremove,
        KeyCode::Char('b') => NormalAction::BundleDump,
        KeyCode::Enter => NormalAction::LoadDetails,
        KeyCode::Char('d') => NormalAction::LoadDepsUses,
        KeyCode::Tab => NormalAction::FocusNext,
        KeyCode::BackTab => NormalAction::FocusPrev,
        KeyCode::Up | KeyCode::Char('k') => NormalAction::ScrollUp,
        KeyCode::Down | KeyCode::Char('j') => NormalAction::ScrollDown,
        KeyCode::Left | KeyCode::Char('l') => NormalAction::PrevStatusTab,
        KeyCode::Right | KeyCode::Char(';') => NormalAction::NextStatusTab,
        _ => return None,
    };

    Some(action)
}

fn handle_normal_mode_key(app: &mut App, key: KeyEvent, channels: &RuntimeChannels) -> KeyOutcome {
    let Some(action) = normal_action_for(key) else {
        return KeyOutcome::Unhandled;
    };

    run_normal_action(app, action, channels)
}

fn run_normal_action(
    app: &mut App,
    action: NormalAction,
    channels: &RuntimeChannels,
) -> KeyOutcome {
    match action {
        NormalAction::Quit => return KeyOutcome::Quit,
        NormalAction::Cancel => {
            if app.has_pending_confirmation() {
                app.clear_pending_confirmations();
                app.set_status("Canceled");
            } else if !app.leaves_query.is_empty() {
                app.leaves_query.clear();
                app.update_all_installed_filters();
                app.set_status("Filters cleared");
            }
        }
        NormalAction::RefreshLists => {
            app.request_leaves(&channels.leaves_tx);
            app.request_casks(&channels.casks_tx);
        }
        NormalAction::CycleTheme => app.cycle_theme(),
        NormalAction::ToggleMouse => app.toggle_mouse(),
        NormalAction::LoadSizes => app.request_sizes(&channels.sizes_tx),
        NormalAction::StatusCheck => app.request_status(&channels.status_tx),
        NormalAction::ToggleView => {
            app.view_mode = match app.view_mode {
                ViewMode::Details => ViewMode::PackageResults,
                ViewMode::PackageResults => ViewMode::Details,
            };
        }
        NormalAction::SearchInstalled => {
            app.input_mode = InputMode::SearchLeaves;
            app.leaves_query.clear();
            app.update_active_installed_filter();
            app.set_status("Search");
        }
        NormalAction::FindPackages => {
            app.input_mode = InputMode::PackageSearch;
            app.package_query.clear();
            app.clear_package_results();
            app.set_status("Search packages");
        }
        NormalAction::ToggleOutdatedFilter => {
            app.clear_pending_confirmations();
            app.toggle_outdated_filter();
            if app.leaves_outdated_only
                && !app.is_cask_mode()
                && app.system_status.is_none()
                && !app.pending_status
            {
                app.request_status(&channels.status_tx);
            }
        }
        NormalAction::ToggleLeavesScope => {
            app.clear_pending_confirmations();
            app.toggle_leaves_scope();
        }
        NormalAction::ToggleInstalledKind => {
            app.clear_pending_confirmations();
            app.toggle_installed_kind();
            if app.is_cask_mode() && app.casks.is_empty() && !app.pending_casks {
                app.request_casks(&channels.casks_tx);
            }
            app.update_active_installed_filter();
        }
        NormalAction::Package(package_action) => {
            if let Some(pkg) = selected_installed_for_action(app, package_action) {
                run_or_confirm_package_action(
                    app,
                    channels,
                    package_action,
                    app.active_package_kind,
                    pkg,
                );
            }
        }
        NormalAction::UpgradeSelectedOrOutdated => {
            if app.focus_panel == FocusedPanel::Status && app.status_tab == StatusTab::Outdated {
                run_or_confirm_upgrade_all_outdated(app, channels);
            } else if let Some(pkg) = selected_installed_for_action(app, PackageAction::Upgrade) {
                run_or_confirm_package_action(
                    app,
                    channels,
                    PackageAction::Upgrade,
                    app.active_package_kind,
                    pkg,
                );
            }
        }
        NormalAction::Service(service_action) => {
            if let Some(service) = selected_service_for_action(app) {
                run_or_confirm_service_action(app, channels, service_action, service);
            }
        }
        NormalAction::ServiceInfo => {
            if let Some(service) = selected_service_for_action(app) {
                app.clear_pending_confirmations();
                app.request_command(
                    CommandKind::ServiceInfo,
                    &["services", "info", &service],
                    &channels.command_tx,
                );
                app.set_status(format!("Loading service info for {service}..."));
            }
        }
        NormalAction::ToggleServicesFailedFilter => {
            if is_services_tab_focused(app) {
                app.clear_pending_confirmations();
                app.toggle_services_failed_filter();
            }
        }
        NormalAction::ToggleServicesAutostartFilter => {
            if is_services_tab_focused(app) {
                app.clear_pending_confirmations();
                app.toggle_services_autostart_filter();
            }
        }
        NormalAction::CycleServicesKindFilter => {
            if is_services_tab_focused(app) {
                app.clear_pending_confirmations();
                app.cycle_services_kind_filter();
            }
        }
        NormalAction::SelfUpdate => {
            if app.pending_self_update {
                app.request_command(
                    CommandKind::SelfUpdate,
                    &["install", "brewery", "--locked", "--force"],
                    &channels.command_tx,
                );
                app.clear_pending_confirmations();
                app.set_status("Updating Brewery...");
            } else {
                app.clear_pending_confirmations();
                app.pending_self_update = true;
                app.set_status(
                    "Update Brewery via `cargo install brewery --locked --force`? [P] confirm, [Esc] cancel",
                );
            }
        }
        NormalAction::Cleanup => {
            app.request_command(
                CommandKind::Cleanup,
                &["cleanup", "-s"],
                &channels.command_tx,
            );
        }
        NormalAction::Autoremove => {
            app.request_command(
                CommandKind::Autoremove,
                &["autoremove"],
                &channels.command_tx,
            );
        }
        NormalAction::BundleDump => {
            app.request_command(
                CommandKind::BundleDump,
                &["bundle", "dump", "--force"],
                &channels.command_tx,
            );
        }
        NormalAction::LoadDetails => app.request_details(DetailsLoad::Basic, &channels.details_tx),
        NormalAction::LoadDepsUses => {
            if app.is_cask_mode() {
                app.set_status("Deps/uses are formula-only");
            } else {
                app.request_details(DetailsLoad::Full, &channels.details_tx);
            }
        }
        NormalAction::FocusNext => app.cycle_focus(),
        NormalAction::FocusPrev => handle_focus_backtab(app),
        NormalAction::ScrollUp => {
            app.scroll_focused_up();
            if app.focus_panel == FocusedPanel::Leaves {
                app.clear_pending_confirmations();
                app.on_selection_change();
            }
        }
        NormalAction::ScrollDown => {
            app.scroll_focused_down();
            if app.focus_panel == FocusedPanel::Leaves {
                app.clear_pending_confirmations();
                app.on_selection_change();
            }
        }
        NormalAction::PrevStatusTab => {
            if app.focus_panel == FocusedPanel::Status {
                app.status_tab_prev();
            }
        }
        NormalAction::NextStatusTab => {
            if app.focus_panel == FocusedPanel::Status {
                app.status_tab_next();
            }
        }
    }

    KeyOutcome::Handled
}

/// The installed package an action should target, reporting why it can't run
/// when the list isn't focused or nothing is selected.
fn selected_installed_for_action(app: &mut App, action: PackageAction) -> Option<String> {
    if app.focus_panel != FocusedPanel::Leaves {
        let noun = app.active_kind_label_singular();
        let verb = action_labels(action).verb;
        app.set_status(format!("Focus {noun} list to {verb}"));
        return None;
    }

    let Some(pkg) = app.selected_installed_package().map(str::to_string) else {
        let noun = app.active_kind_label_singular();
        app.set_status(format!("No {noun} selected"));
        return None;
    };

    Some(pkg)
}

/// The service an action should target. Service bindings are silent outside the
/// Services tab, since they are meaningless there.
fn selected_service_for_action(app: &mut App) -> Option<String> {
    if !is_services_tab_focused(app) {
        return None;
    }

    let Some(service) = app.selected_service().map(str::to_string) else {
        app.set_status("No service selected");
        return None;
    };

    Some(service)
}

fn is_services_tab_focused(app: &App) -> bool {
    app.focus_panel == FocusedPanel::Status && app.status_tab == StatusTab::Services
}

fn handle_search_leaves_mode_key(app: &mut App, key: KeyEvent) -> KeyOutcome {
    match key.code {
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            app.set_status("Ready");
        }
        KeyCode::Esc => {
            if !app.leaves_query.is_empty() {
                app.leaves_query.clear();
                app.update_active_installed_filter();
            }
            app.input_mode = InputMode::Normal;
            app.set_status("Ready");
        }
        KeyCode::Up => {
            app.select_prev();
            app.on_selection_change();
        }
        KeyCode::Down => {
            app.select_next();
            app.on_selection_change();
        }
        KeyCode::Backspace => {
            app.leaves_query.pop();
            app.update_active_installed_filter();
        }
        KeyCode::Char(ch) => {
            app.leaves_query.push(ch);
            app.update_active_installed_filter();
        }
        _ => return KeyOutcome::Unhandled,
    }

    KeyOutcome::Handled
}

fn handle_package_search_mode_key(
    app: &mut App,
    key: KeyEvent,
    channels: &RuntimeChannels,
) -> KeyOutcome {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.package_query.clear();
            app.clear_package_results();
            app.set_status("Ready");
        }
        KeyCode::Up => {
            app.select_prev_result();
            app.on_selection_change();
        }
        KeyCode::Down => {
            app.select_next_result();
            app.on_selection_change();
        }
        KeyCode::Enter => {
            let query = app.package_query.trim().to_string();
            if query.is_empty() {
                app.set_status("Enter a package name");
                return KeyOutcome::Handled;
            }

            app.request_command(
                CommandKind::Search,
                &["search", &query],
                &channels.command_tx,
            );
            app.last_package_search = Some(query);
            app.set_status("Searching...");
        }
        KeyCode::Backspace => {
            app.package_query.pop();
            app.clear_package_results();
        }
        KeyCode::Char(ch) => {
            app.package_query.push(ch);
            app.clear_package_results();
        }
        _ => return KeyOutcome::Unhandled,
    }

    KeyOutcome::Handled
}

fn handle_package_results_mode_key(
    app: &mut App,
    key: KeyEvent,
    channels: &RuntimeChannels,
) -> KeyOutcome {
    match key.code {
        KeyCode::Esc => {
            if app.has_pending_confirmation() {
                app.clear_pending_confirmations();
                app.set_status("Canceled");
            } else {
                app.input_mode = InputMode::Normal;
                app.package_query.clear();
                app.clear_package_results();
                app.set_status("Ready");
            }
        }
        KeyCode::Char('f') => {
            app.input_mode = InputMode::PackageSearch;
            app.package_query.clear();
            app.clear_package_results();
            app.clear_pending_confirmations();
            app.set_status("Search packages");
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_prev_result();
            app.clear_pending_confirmations();
            app.on_selection_change();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next_result();
            app.clear_pending_confirmations();
            app.on_selection_change();
        }
        KeyCode::Char('i') => {
            let Some(pkg) = app.selected_package_result().map(str::to_string) else {
                app.set_status("No result selected");
                return KeyOutcome::Handled;
            };
            run_or_confirm_package_action(
                app,
                channels,
                PackageAction::Install,
                PackageKind::Formula,
                pkg,
            );
        }
        KeyCode::Char('u') => {
            let Some(pkg) = app.selected_package_result().map(str::to_string) else {
                app.set_status("No result selected");
                return KeyOutcome::Handled;
            };
            run_or_confirm_package_action(
                app,
                channels,
                PackageAction::Uninstall,
                PackageKind::Formula,
                pkg,
            );
        }
        _ => return KeyOutcome::Unhandled,
    }

    KeyOutcome::Handled
}

fn run_or_confirm_package_action(
    app: &mut App,
    channels: &RuntimeChannels,
    action: PackageAction,
    kind: PackageKind,
    pkg: String,
) {
    let labels = action_labels(action);
    let noun = package_kind_noun(kind);

    if matches!(app.pending_package_action.as_ref(), Some(pending) if pending.action == action && pending.kind == kind && pending.pkg == pkg)
    {
        let args = package_action_args(action, kind, &pkg);
        app.request_command(labels.command, &args, &channels.command_tx);
        app.clear_pending_confirmations();
        app.set_status(format!("{} {noun}...", labels.verb_ing));
        return;
    }

    // Uninstalls can silently orphan dependencies, so say so before confirming.
    let impact = match action {
        PackageAction::Uninstall => app
            .removal_impact_hint(&pkg)
            .map(|hint| format!(" {hint}"))
            .unwrap_or_default(),
        PackageAction::Install | PackageAction::Upgrade => String::new(),
    };

    let confirmation_status = format!(
        "{} {noun} {pkg}?{impact} [{}] confirm, [Esc] cancel",
        labels.verb_title, labels.confirm_key
    );
    app.pending_upgrade_all_outdated = false;
    app.pending_package_action = Some(PendingPackageAction { action, kind, pkg });
    app.set_status(confirmation_status);
}

fn run_or_confirm_upgrade_all_outdated(app: &mut App, channels: &RuntimeChannels) {
    let outdated = app
        .system_status
        .as_ref()
        .map_or(0, |status| status.outdated_packages.len());
    if outdated == 0 {
        app.set_status("No outdated packages");
        return;
    }

    if app.pending_upgrade_all_outdated {
        app.request_command(CommandKind::UpgradeAll, &["upgrade"], &channels.command_tx);
        app.clear_pending_confirmations();
        app.set_status(format!("Upgrading {outdated} outdated packages..."));
        return;
    }

    app.pending_package_action = None;
    app.pending_upgrade_all_outdated = true;
    app.set_status(format!(
        "Upgrade all {outdated} outdated packages? [U] confirm, [Esc] cancel"
    ));
}

fn run_or_confirm_service_action(
    app: &mut App,
    channels: &RuntimeChannels,
    action: ServiceAction,
    service: String,
) {
    let labels = service_action_labels(action);

    if matches!(app.pending_service_action.as_ref(), Some(pending) if pending.action == action && pending.service == service)
    {
        let args = service_action_args(action, &service);
        app.request_command(labels.command, &args, &channels.command_tx);
        app.clear_pending_confirmations();
        app.set_status(format!("{} service...", labels.verb_ing));
        return;
    }

    let confirmation_status = format!(
        "{} service {service}? [{}] confirm, [Esc] cancel",
        labels.verb_title, labels.confirm_key
    );
    app.pending_package_action = None;
    app.pending_upgrade_all_outdated = false;
    app.pending_service_action = Some(PendingServiceAction { action, service });
    app.set_status(confirmation_status);
}

/// How an action is named across the confirmation prompt, the in-progress
/// status, and the "focus the list first" hint.
struct ActionLabels {
    command: CommandKind,
    verb: &'static str,
    verb_ing: &'static str,
    verb_title: &'static str,
    confirm_key: char,
}

fn action_labels(action: PackageAction) -> ActionLabels {
    match action {
        PackageAction::Install => ActionLabels {
            command: CommandKind::Install,
            verb: "install",
            verb_ing: "Installing",
            verb_title: "Install",
            confirm_key: 'i',
        },
        PackageAction::Uninstall => ActionLabels {
            command: CommandKind::Uninstall,
            verb: "uninstall",
            verb_ing: "Uninstalling",
            verb_title: "Uninstall",
            confirm_key: 'u',
        },
        PackageAction::Upgrade => ActionLabels {
            command: CommandKind::Upgrade,
            verb: "upgrade",
            verb_ing: "Upgrading",
            verb_title: "Upgrade",
            confirm_key: 'U',
        },
    }
}

fn service_action_labels(action: ServiceAction) -> ActionLabels {
    match action {
        ServiceAction::Start => ActionLabels {
            command: CommandKind::ServiceStart,
            verb: "start",
            verb_ing: "Starting",
            verb_title: "Start",
            confirm_key: 'S',
        },
        ServiceAction::Stop => ActionLabels {
            command: CommandKind::ServiceStop,
            verb: "stop",
            verb_ing: "Stopping",
            verb_title: "Stop",
            confirm_key: 'X',
        },
        ServiceAction::Restart => ActionLabels {
            command: CommandKind::ServiceRestart,
            verb: "restart",
            verb_ing: "Restarting",
            verb_title: "Restart",
            confirm_key: 'R',
        },
    }
}

fn package_action_args(action: PackageAction, kind: PackageKind, pkg: &str) -> Vec<&str> {
    match (action, kind) {
        (PackageAction::Install, PackageKind::Formula) => vec!["install", pkg],
        (PackageAction::Install, PackageKind::Cask) => vec!["install", "--cask", pkg],
        (PackageAction::Uninstall, PackageKind::Formula) => vec!["uninstall", pkg],
        (PackageAction::Uninstall, PackageKind::Cask) => vec!["uninstall", "--cask", pkg],
        (PackageAction::Upgrade, PackageKind::Formula) => vec!["upgrade", pkg],
        (PackageAction::Upgrade, PackageKind::Cask) => vec!["upgrade", "--cask", pkg],
    }
}

fn package_kind_noun(kind: PackageKind) -> &'static str {
    match kind {
        PackageKind::Formula => "formula",
        PackageKind::Cask => "cask",
    }
}

fn service_action_args(action: ServiceAction, service: &str) -> Vec<&str> {
    match action {
        ServiceAction::Start => vec!["services", "start", service],
        ServiceAction::Stop => vec!["services", "stop", service],
        ServiceAction::Restart => vec!["services", "restart", service],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The help popup replays the key it advertises, so an advertised binding
    /// the dispatcher does not recognize would silently do nothing.
    #[test]
    fn every_advertised_keymap_is_dispatchable() {
        for command in keymap::commands() {
            let key = command.key_event();
            assert!(
                global_action_for(key).is_some() || normal_action_for(key).is_some(),
                "`{}` ({}) is advertised in the help popup but no handler recognizes it",
                command.label(true),
                command.description,
            );
        }
    }

    /// Guards against an arm being shadowed by an earlier one during edits.
    #[test]
    fn advertised_keymaps_map_to_distinct_actions() {
        let mut seen: Vec<NormalAction> = Vec::new();
        for command in keymap::commands() {
            let key = command.key_event();
            // Global keymaps are dispatched first, so they never reach
            // normal-mode recognition even when a code matches.
            if global_action_for(key).is_some() {
                continue;
            }
            let Some(action) = normal_action_for(key) else {
                continue;
            };
            assert!(
                !seen.contains(&action),
                "`{}` resolves to {action:?}, which another binding already claims",
                command.label(true),
            );
            seen.push(action);
        }
    }

    #[test]
    fn unbound_keys_are_reported_as_unhandled() {
        let key = KeyEvent::new(KeyCode::Char('~'), KeyModifiers::NONE);
        assert!(normal_action_for(key).is_none());
        assert!(global_action_for(key).is_none());
    }

    #[test]
    fn arrow_keys_mirror_their_vim_bindings() {
        let plain = |code| KeyEvent::new(code, KeyModifiers::NONE);
        for (arrow, letter) in [
            (KeyCode::Up, 'k'),
            (KeyCode::Down, 'j'),
            (KeyCode::Left, 'l'),
            (KeyCode::Right, ';'),
        ] {
            assert_eq!(
                normal_action_for(plain(arrow)),
                normal_action_for(plain(KeyCode::Char(letter))),
                "{arrow:?} and '{letter}' should trigger the same action",
            );
        }
    }

    #[test]
    fn alt_i_toggles_icons_but_plain_i_installs() {
        assert_eq!(
            global_action_for(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::ALT)),
            Some(GlobalAction::ToggleIcons),
        );
        assert!(global_action_for(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)).is_none());
        assert_eq!(
            normal_action_for(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
            Some(NormalAction::Package(PackageAction::Install)),
        );
    }
}
