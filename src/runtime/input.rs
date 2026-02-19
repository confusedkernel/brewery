use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{
    App, FocusedPanel, InputMode, PackageAction, PackageKind, PendingPackageAction, StatusTab,
    ViewMode,
};
use crate::brew::{CommandKind, DetailsLoad};
use crate::runtime::messages::{RuntimeChannels, handle_focus_backtab};

pub fn handle_key_event(
    app: &mut App,
    key: KeyEvent,
    channels: &RuntimeChannels,
    help_max_offset: usize,
) -> Option<anyhow::Result<()>> {
    app.needs_redraw = true;

    if handle_global_keymaps(app, key) {
        return None;
    }

    if handle_help_popup_input(app, key, help_max_offset) {
        return None;
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode_key(app, key, channels),
        InputMode::SearchLeaves => handle_search_leaves_mode_key(app, key),
        InputMode::PackageSearch => handle_package_search_mode_key(app, key, channels),
        InputMode::PackageResults => handle_package_results_mode_key(app, key, channels),
    }
}

fn handle_global_keymaps(app: &mut App, key: KeyEvent) -> bool {
    if app.input_mode != InputMode::Normal {
        return false;
    }

    if key.code == KeyCode::Char('?') {
        app.toggle_help();
        return true;
    }

    if key.code == KeyCode::Char('i') && key.modifiers.contains(KeyModifiers::ALT) {
        app.toggle_icons();
        return true;
    }

    false
}

fn handle_help_popup_input(app: &mut App, key: KeyEvent, help_max_offset: usize) -> bool {
    if !app.show_help_popup {
        return false;
    }

    match key.code {
        KeyCode::Esc => {
            app.show_help_popup = false;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.help_scroll_offset < help_max_offset {
                app.help_scroll_offset += 1;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.help_scroll_offset = app.help_scroll_offset.saturating_sub(1);
        }
        _ => {}
    }

    true
}

fn handle_normal_mode_key(
    app: &mut App,
    key: KeyEvent,
    channels: &RuntimeChannels,
) -> Option<anyhow::Result<()>> {
    match key.code {
        KeyCode::Char('q') => Some(Ok(())),
        KeyCode::Esc => {
            if has_pending_confirmation(app) {
                clear_pending_confirmations(app);
                set_status(app, "Canceled");
            } else if !app.leaves_query.is_empty() {
                app.leaves_query.clear();
                app.update_all_installed_filters();
                set_status(app, "Filters cleared");
            }
            None
        }
        KeyCode::Char('r') => {
            app.request_leaves(&channels.leaves_tx);
            app.request_casks(&channels.casks_tx);
            None
        }
        KeyCode::Char('t') => {
            app.cycle_theme();
            None
        }
        KeyCode::Char('s') => {
            app.request_sizes(&channels.sizes_tx);
            None
        }
        KeyCode::Char('h') => {
            app.request_status(&channels.status_tx);
            None
        }
        KeyCode::Char('v') => {
            app.view_mode = match app.view_mode {
                ViewMode::Details => ViewMode::PackageResults,
                ViewMode::PackageResults => ViewMode::Details,
            };
            None
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::SearchLeaves;
            app.leaves_query.clear();
            app.update_active_installed_filter();
            set_status(app, "Search");
            None
        }
        KeyCode::Char('o') => {
            clear_pending_confirmations(app);
            app.toggle_outdated_filter();
            if app.leaves_outdated_only
                && !app.is_cask_mode()
                && app.system_status.is_none()
                && !app.pending_status
            {
                app.request_status(&channels.status_tx);
            }
            None
        }
        KeyCode::Char('C') => {
            clear_pending_confirmations(app);
            app.toggle_installed_kind();
            if app.is_cask_mode() && app.casks.is_empty() && !app.pending_casks {
                app.request_casks(&channels.casks_tx);
            }
            app.update_active_installed_filter();
            None
        }
        KeyCode::Char('f') => {
            app.input_mode = InputMode::PackageSearch;
            app.package_query.clear();
            app.clear_package_results();
            set_status(app, "Search packages");
            None
        }
        KeyCode::Char('i') => {
            if app.focus_panel == FocusedPanel::Leaves {
                let Some(pkg) = app.selected_installed_package().map(str::to_string) else {
                    set_status(
                        app,
                        format!("No {} selected", app.active_kind_label_singular()),
                    );
                    return None;
                };
                run_or_confirm_package_action(
                    app,
                    channels,
                    PackageAction::Install,
                    app.active_package_kind,
                    pkg,
                );
            } else {
                set_status(
                    app,
                    format!("Focus {} list to install", app.active_kind_label_singular()),
                );
            }
            None
        }
        KeyCode::Char('u') => {
            if app.focus_panel == FocusedPanel::Leaves {
                let Some(pkg) = app.selected_installed_package().map(str::to_string) else {
                    set_status(
                        app,
                        format!("No {} selected", app.active_kind_label_singular()),
                    );
                    return None;
                };
                run_or_confirm_package_action(
                    app,
                    channels,
                    PackageAction::Uninstall,
                    app.active_package_kind,
                    pkg,
                );
            } else {
                set_status(
                    app,
                    format!(
                        "Focus {} list to uninstall",
                        app.active_kind_label_singular()
                    ),
                );
            }
            None
        }
        KeyCode::Char('U') => {
            if app.focus_panel == FocusedPanel::Status && app.status_tab == StatusTab::Outdated {
                run_or_confirm_upgrade_all_outdated(app, channels);
            } else if app.focus_panel == FocusedPanel::Leaves {
                let Some(pkg) = app.selected_installed_package().map(str::to_string) else {
                    set_status(
                        app,
                        format!("No {} selected", app.active_kind_label_singular()),
                    );
                    return None;
                };
                run_or_confirm_package_action(
                    app,
                    channels,
                    PackageAction::Upgrade,
                    app.active_package_kind,
                    pkg,
                );
            } else {
                set_status(
                    app,
                    format!("Focus {} list to upgrade", app.active_kind_label_singular()),
                );
            }
            None
        }
        KeyCode::Char('c') => {
            app.request_command(
                CommandKind::Cleanup,
                &["cleanup", "-s"],
                &channels.command_tx,
            );
            None
        }
        KeyCode::Char('a') => {
            app.request_command(
                CommandKind::Autoremove,
                &["autoremove"],
                &channels.command_tx,
            );
            None
        }
        KeyCode::Char('b') => {
            app.request_command(
                CommandKind::BundleDump,
                &["bundle", "dump", "--force"],
                &channels.command_tx,
            );
            None
        }
        KeyCode::Char('P') => {
            if app.pending_self_update {
                app.request_command(
                    CommandKind::SelfUpdate,
                    &["install", "brewery", "--locked", "--force"],
                    &channels.command_tx,
                );
                clear_pending_confirmations(app);
                set_status(app, "Updating Brewery...");
            } else {
                clear_pending_confirmations(app);
                app.pending_self_update = true;
                set_status(
                    app,
                    "Update Brewery via `cargo install brewery --locked --force`? [P] confirm, [Esc] cancel",
                );
            }
            None
        }
        KeyCode::Enter => {
            app.request_details(DetailsLoad::Basic, &channels.details_tx);
            None
        }
        KeyCode::Char('d') => {
            if app.is_cask_mode() {
                set_status(app, "Deps/uses are formula-only");
                return None;
            }
            app.request_details(DetailsLoad::Full, &channels.details_tx);
            None
        }
        KeyCode::Tab => {
            app.cycle_focus();
            None
        }
        KeyCode::BackTab => {
            handle_focus_backtab(app);
            None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.scroll_focused_up();
            if app.focus_panel == FocusedPanel::Leaves {
                clear_pending_confirmations(app);
                app.on_selection_change();
            }
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.scroll_focused_down();
            if app.focus_panel == FocusedPanel::Leaves {
                clear_pending_confirmations(app);
                app.on_selection_change();
            }
            None
        }
        KeyCode::Left | KeyCode::Char('l') if app.focus_panel == FocusedPanel::Status => {
            app.status_tab_prev();
            None
        }
        KeyCode::Right | KeyCode::Char(';') if app.focus_panel == FocusedPanel::Status => {
            app.status_tab_next();
            None
        }
        _ => None,
    }
}

fn handle_search_leaves_mode_key(app: &mut App, key: KeyEvent) -> Option<anyhow::Result<()>> {
    match key.code {
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            set_status(app, "Ready");
        }
        KeyCode::Esc => {
            if !app.leaves_query.is_empty() {
                app.leaves_query.clear();
                app.update_active_installed_filter();
            }
            app.input_mode = InputMode::Normal;
            set_status(app, "Ready");
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
        _ => {}
    }
    None
}

fn handle_package_search_mode_key(
    app: &mut App,
    key: KeyEvent,
    channels: &RuntimeChannels,
) -> Option<anyhow::Result<()>> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.package_query.clear();
            app.clear_package_results();
            set_status(app, "Ready");
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
                set_status(app, "Enter a package name");
                return None;
            }

            app.request_command(
                CommandKind::Search,
                &["search", &query],
                &channels.command_tx,
            );
            app.last_package_search = Some(query);
            set_status(app, "Searching...");
        }
        KeyCode::Backspace => {
            app.package_query.pop();
            app.clear_package_results();
        }
        KeyCode::Char(ch) => {
            app.package_query.push(ch);
            app.clear_package_results();
        }
        _ => {}
    }
    None
}

fn handle_package_results_mode_key(
    app: &mut App,
    key: KeyEvent,
    channels: &RuntimeChannels,
) -> Option<anyhow::Result<()>> {
    match key.code {
        KeyCode::Esc => {
            if has_pending_confirmation(app) {
                clear_pending_confirmations(app);
                set_status(app, "Canceled");
            } else {
                app.input_mode = InputMode::Normal;
                app.package_query.clear();
                app.clear_package_results();
                set_status(app, "Ready");
            }
        }
        KeyCode::Char('f') => {
            app.input_mode = InputMode::PackageSearch;
            app.package_query.clear();
            app.clear_package_results();
            clear_pending_confirmations(app);
            set_status(app, "Search packages");
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_prev_result();
            clear_pending_confirmations(app);
            app.on_selection_change();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next_result();
            clear_pending_confirmations(app);
            app.on_selection_change();
        }
        KeyCode::Char('i') => {
            let Some(pkg) = app.selected_package_result().map(str::to_string) else {
                set_status(app, "No result selected");
                return None;
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
                set_status(app, "No result selected");
                return None;
            };
            run_or_confirm_package_action(
                app,
                channels,
                PackageAction::Uninstall,
                PackageKind::Formula,
                pkg,
            );
        }
        _ => {}
    }
    None
}

fn run_or_confirm_package_action(
    app: &mut App,
    channels: &RuntimeChannels,
    action: PackageAction,
    kind: PackageKind,
    pkg: String,
) {
    let (command_kind, verb_ing, verb_title, confirm_key) = package_action_labels(action);
    let noun = package_kind_noun(kind);

    if matches!(app.pending_package_action.as_ref(), Some(pending) if pending.action == action && pending.kind == kind && pending.pkg == pkg)
    {
        let args = package_action_args(action, kind, &pkg);
        app.request_command(command_kind, &args, &channels.command_tx);
        clear_pending_confirmations(app);
        set_status(app, format!("{verb_ing} {noun}..."));
        return;
    }

    let confirmation_status =
        format!("{verb_title} {noun} {pkg}? [{confirm_key}] confirm, [Esc] cancel");
    app.pending_upgrade_all_outdated = false;
    app.pending_package_action = Some(PendingPackageAction { action, kind, pkg });
    set_status(app, confirmation_status);
}

fn run_or_confirm_upgrade_all_outdated(app: &mut App, channels: &RuntimeChannels) {
    let outdated = app
        .system_status
        .as_ref()
        .map_or(0, |status| status.outdated_packages.len());
    if outdated == 0 {
        set_status(app, "No outdated packages");
        return;
    }

    if app.pending_upgrade_all_outdated {
        app.request_command(CommandKind::UpgradeAll, &["upgrade"], &channels.command_tx);
        clear_pending_confirmations(app);
        set_status(app, format!("Upgrading {outdated} outdated packages..."));
        return;
    }

    app.pending_package_action = None;
    app.pending_upgrade_all_outdated = true;
    set_status(
        app,
        format!("Upgrade all {outdated} outdated packages? [U] confirm, [Esc] cancel"),
    );
}

fn package_action_labels(action: PackageAction) -> (CommandKind, &'static str, &'static str, char) {
    match action {
        PackageAction::Install => (CommandKind::Install, "Installing", "Install", 'i'),
        PackageAction::Uninstall => (CommandKind::Uninstall, "Uninstalling", "Uninstall", 'u'),
        PackageAction::Upgrade => (CommandKind::Upgrade, "Upgrading", "Upgrade", 'U'),
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

fn has_pending_confirmation(app: &App) -> bool {
    app.pending_package_action.is_some()
        || app.pending_upgrade_all_outdated
        || app.pending_self_update
}

fn clear_pending_confirmations(app: &mut App) {
    app.pending_package_action = None;
    app.pending_upgrade_all_outdated = false;
    app.pending_self_update = false;
}

fn set_status(app: &mut App, status: impl Into<String>) {
    app.status = status.into();
    app.last_refresh = Instant::now();
}
