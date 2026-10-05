use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{
    App, Confirmation, FocusedPanel, InputMode, PackageAction, PackageKind, PendingPackageAction,
    PendingServiceAction, ServiceAction, StatusTab, ViewMode,
};
use crate::brew::{CommandKind, DetailsLoad};
use crate::runtime::messages::RuntimeChannels;
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
    CycleSort,
    Package(PackageAction),
    TogglePin,
    OpenHomepage,
    BrewUpdate,
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
        KeyCode::Char('O') => NormalAction::CycleSort,
        KeyCode::Char('p') => NormalAction::TogglePin,
        KeyCode::Char('g') => NormalAction::OpenHomepage,
        KeyCode::Char('e') => NormalAction::BrewUpdate,
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
            app.request_leaves(&channels.tx);
            app.request_casks(&channels.tx);
        }
        NormalAction::CycleTheme => app.cycle_theme(),
        NormalAction::ToggleMouse => app.toggle_mouse(),
        NormalAction::LoadSizes => app.request_sizes(&channels.tx),
        NormalAction::StatusCheck => app.request_status(&channels.tx),
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
                && !app.status_job.is_running()
            {
                app.request_status(&channels.tx);
            }
        }
        NormalAction::ToggleLeavesScope => {
            app.clear_pending_confirmations();
            app.toggle_leaves_scope();
        }
        NormalAction::CycleSort => {
            app.clear_pending_confirmations();
            app.cycle_sort_mode();
        }
        NormalAction::TogglePin => {
            if app.is_cask_mode() {
                app.set_status("Pinning is formula-only");
            } else if let Some(pkg) = selected_installed_for_action(app, "pin") {
                toggle_pin(app, channels, pkg);
            }
        }
        NormalAction::OpenHomepage => open_homepage(app, channels),
        NormalAction::BrewUpdate => {
            app.clear_pending_confirmations();
            app.request_command(CommandKind::Update, &["update"], &channels.tx);
            app.set_status("Running brew update...");
        }
        NormalAction::ToggleInstalledKind => {
            app.clear_pending_confirmations();
            app.toggle_installed_kind();
            if app.is_cask_mode() && app.casks.is_empty() && !app.casks_job.is_running() {
                app.request_casks(&channels.tx);
            }
            app.update_active_installed_filter();
        }
        NormalAction::Package(package_action) => {
            if let Some(pkg) =
                selected_installed_for_action(app, action_labels(package_action).verb)
            {
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
            } else if let Some(pkg) =
                selected_installed_for_action(app, action_labels(PackageAction::Upgrade).verb)
            {
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
                    &channels.tx,
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
            if confirm(
                app,
                Confirmation::SelfUpdate,
                "Update Brewery via `cargo install brewery --locked --force`? [P] confirm, [Esc] cancel",
            ) {
                app.request_command(
                    CommandKind::SelfUpdate,
                    &["install", "brewery", "--locked", "--force"],
                    &channels.tx,
                );
                app.set_status("Updating Brewery...");
            }
        }
        NormalAction::Cleanup => {
            app.request_command(CommandKind::Cleanup, &["cleanup", "-s"], &channels.tx);
        }
        NormalAction::Autoremove => run_or_confirm_autoremove(app, channels),
        NormalAction::BundleDump => {
            app.request_command(
                CommandKind::BundleDump,
                &["bundle", "dump", "--force"],
                &channels.tx,
            );
        }
        NormalAction::LoadDetails => app.request_details(DetailsLoad::Basic, &channels.tx),
        NormalAction::LoadDepsUses => {
            if app.is_cask_mode() {
                app.set_status("Deps/uses are formula-only");
            } else {
                app.request_details(DetailsLoad::Full, &channels.tx);
            }
        }
        NormalAction::FocusNext => app.cycle_focus(),
        NormalAction::FocusPrev => app.cycle_focus_back(),
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
                app.select_status_tab(app.status_tab.prev());
            }
        }
        NormalAction::NextStatusTab => {
            if app.focus_panel == FocusedPanel::Status {
                app.select_status_tab(app.status_tab.next());
            }
        }
    }

    KeyOutcome::Handled
}

/// The installed package an action should target, reporting why it can't run
/// when the list isn't focused or nothing is selected.
fn selected_installed_for_action(app: &mut App, verb: &str) -> Option<String> {
    if app.focus_panel != FocusedPanel::Leaves {
        let noun = app.active_package_kind.noun();
        app.set_status(format!("Focus {noun} list to {verb}"));
        return None;
    }

    let Some(pkg) = app.selected_installed_package().map(str::to_string) else {
        let noun = app.active_package_kind.noun();
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

            app.request_command(CommandKind::Search, &["search", &query], &channels.tx);
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
        KeyCode::Char(key @ ('i' | 'u')) => {
            let action = if key == 'i' {
                PackageAction::Install
            } else {
                PackageAction::Uninstall
            };
            match app.selected_package_result().map(str::to_string) {
                Some(pkg) => {
                    run_or_confirm_package_action(app, channels, action, PackageKind::Formula, pkg)
                }
                None => app.set_status("No result selected"),
            }
        }
        _ => return KeyOutcome::Unhandled,
    }

    KeyOutcome::Handled
}

/// Arms `pending` on the first press, showing `prompt`. Returns true on the
/// second press of the same action, which the caller should then run.
fn confirm(app: &mut App, pending: Confirmation, prompt: impl Into<String>) -> bool {
    if app.pending_confirmation.as_ref() == Some(&pending) {
        app.clear_pending_confirmations();
        return true;
    }

    app.pending_confirmation = Some(pending);
    app.set_status(prompt);
    false
}

fn run_or_confirm_package_action(
    app: &mut App,
    channels: &RuntimeChannels,
    action: PackageAction,
    kind: PackageKind,
    pkg: String,
) {
    let labels = action_labels(action);
    let noun = kind.noun();

    // Uninstalls can silently orphan dependencies, so say so before confirming.
    let impact = match action {
        PackageAction::Uninstall => app
            .removal_impact_hint(&pkg)
            .map(|hint| format!(" {hint}"))
            .unwrap_or_default(),
        PackageAction::Install | PackageAction::Upgrade => String::new(),
    };
    let prompt = format!(
        "{} {noun} {pkg}?{impact} [{}] confirm, [Esc] cancel",
        labels.verb_title, labels.confirm_key
    );

    let pending = PendingPackageAction {
        action,
        kind,
        pkg: pkg.clone(),
    };
    if confirm(app, Confirmation::Package(pending), prompt) {
        let mut args = vec![labels.verb];
        if kind == PackageKind::Cask {
            args.push("--cask");
        }
        args.push(&pkg);
        app.request_command(labels.command, &args, &channels.tx);
        app.set_status(format!("{} {noun}...", labels.verb_ing));
    }
}

/// Pinning is not destructive, so it runs on the first press. The status line
/// says which way it went, since the list marker takes a status refresh to
/// catch up.
fn toggle_pin(app: &mut App, channels: &RuntimeChannels, pkg: String) {
    app.clear_pending_confirmations();
    if app.is_pinned(&pkg) {
        app.request_command(CommandKind::Unpin, &["unpin", &pkg], &channels.tx);
        app.set_status(format!("Unpinning {pkg}..."));
    } else {
        app.request_command(CommandKind::Pin, &["pin", &pkg], &channels.tx);
        app.set_status(format!("Pinning {pkg}..."));
    }
}

/// Hands the selected package's homepage to the desktop. Works for search
/// results as well as installed packages, as long as details have loaded.
fn open_homepage(app: &mut App, channels: &RuntimeChannels) {
    let Some(pkg) = app.selected_package_name().map(str::to_string) else {
        app.set_status("No package selected");
        return;
    };

    let Some(details) = app.details_cache.peek(&pkg) else {
        app.set_status("Load details first (Enter)");
        return;
    };
    let Some(url) = details.homepage.clone() else {
        app.set_status(format!("{pkg} has no homepage"));
        return;
    };

    app.clear_pending_confirmations();
    app.request_command(CommandKind::OpenHomepage, &[&url], &channels.tx);
    app.set_status(format!("Opening {url}"));
}

/// `brew autoremove` deletes without asking, so it gets the same two-step
/// confirmation as an uninstall, with the orphan set previewed in Details.
fn run_or_confirm_autoremove(app: &mut App, channels: &RuntimeChannels) {
    let armed = app.pending_confirmation == Some(Confirmation::Autoremove);

    // With the graph loaded the answer is known; skip the prompt when it is
    // "nothing" rather than confirming a no-op.
    let prompt = match app.autoremove_impact() {
        Some(impact) if impact.is_empty() && !armed => {
            app.set_status("Nothing to autoremove");
            return;
        }
        Some(_) => format!(
            "Autoremove {}? [a] confirm, [Esc] cancel",
            app.autoremove_hint().unwrap_or_default()
        ),
        None => "Autoremove unused dependencies? [a] confirm, [Esc] cancel".to_string(),
    };

    if confirm(app, Confirmation::Autoremove, prompt) {
        app.request_command(CommandKind::Autoremove, &["autoremove"], &channels.tx);
        app.set_status("Removing unused dependencies...");
    }
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

    let prompt = format!("Upgrade all {outdated} outdated packages? [U] confirm, [Esc] cancel");
    if confirm(app, Confirmation::UpgradeAllOutdated, prompt) {
        app.request_command(CommandKind::UpgradeAll, &["upgrade"], &channels.tx);
        app.set_status(format!("Upgrading {outdated} outdated packages..."));
    }
}

fn run_or_confirm_service_action(
    app: &mut App,
    channels: &RuntimeChannels,
    action: ServiceAction,
    service: String,
) {
    let labels = service_action_labels(action);
    let prompt = format!(
        "{} service {service}? [{}] confirm, [Esc] cancel",
        labels.verb_title, labels.confirm_key
    );

    let pending = PendingServiceAction {
        action,
        service: service.clone(),
    };
    if confirm(app, Confirmation::Service(pending), prompt) {
        app.request_command(
            labels.command,
            &["services", labels.verb, &service],
            &channels.tx,
        );
        app.set_status(format!("{} service...", labels.verb_ing));
    }
}

/// How an action is named across the confirmation prompt, the in-progress
/// status, and the "focus the list first" hint. `verb` doubles as the `brew`
/// subcommand.
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
    fn new_bindings_resolve_to_their_actions() {
        let plain = |ch| KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
        assert_eq!(normal_action_for(plain('p')), Some(NormalAction::TogglePin));
        assert_eq!(
            normal_action_for(plain('g')),
            Some(NormalAction::OpenHomepage)
        );
        assert_eq!(
            normal_action_for(plain('e')),
            Some(NormalAction::BrewUpdate)
        );
        assert_eq!(normal_action_for(plain('O')), Some(NormalAction::CycleSort));
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
