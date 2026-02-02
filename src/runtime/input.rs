use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, FocusedPanel, InputMode, PackageAction, PendingPackageAction, ViewMode};
use crate::brew::DetailsLoad;
use crate::runtime::messages::{handle_focus_backtab, RuntimeChannels};

pub fn handle_key_event(
    app: &mut App,
    key: KeyEvent,
    channels: &RuntimeChannels,
    help_max_offset: usize,
) -> Option<anyhow::Result<()>> {
    // Any keypress should trigger a redraw
    app.needs_redraw = true;

    // Handle global keymaps only in Normal mode
    if app.input_mode == InputMode::Normal {
        if key.code == KeyCode::Char('?') {
            app.toggle_help();
            return None;
        }

        if key.code == KeyCode::Char('i') && key.modifiers.contains(KeyModifiers::ALT) {
            app.toggle_icons();
            return None;
        }
    }

    // Close help popup with Esc
    if app.show_help_popup {
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
        return None;
    }

    match app.input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => return Some(Ok(())),
            KeyCode::Esc => {
                if app.pending_package_action.is_some() {
                    app.pending_package_action = None;
                    app.status = "Canceled".to_string();
                    app.last_refresh = Instant::now();
                } else if !app.leaves_query.is_empty() {
                    app.leaves_query.clear();
                    app.update_filtered_leaves();
                    app.status = "Filters cleared".to_string();
                    app.last_refresh = Instant::now();
                }
            }
            KeyCode::Char('r') => app.request_leaves(&channels.leaves_tx),
            KeyCode::Char('t') => app.cycle_theme(),
            KeyCode::Char('s') => app.request_sizes(&channels.sizes_tx),
            KeyCode::Char('h') => app.request_health(&channels.health_tx),
            KeyCode::Char('v') => {
                app.view_mode = match app.view_mode {
                    ViewMode::Details => ViewMode::PackageResults,
                    ViewMode::PackageResults => ViewMode::Details,
                };
            }
            KeyCode::Char('/') => {
                app.input_mode = InputMode::SearchLeaves;
                app.leaves_query.clear();
                app.update_filtered_leaves();
                app.status = "Search".to_string();
                app.last_refresh = std::time::Instant::now();
            }
            KeyCode::Char('f') => {
                app.input_mode = InputMode::PackageSearch;
                app.package_query.clear();
                app.clear_package_results();
                app.status = "Search packages".to_string();
                app.last_refresh = std::time::Instant::now();
            }
            KeyCode::Char('i') => {
                if app.focus_panel == FocusedPanel::Leaves {
                    let Some(pkg) = app.selected_leaf().map(str::to_string) else {
                        app.status = "No leaf selected".to_string();
                        app.last_refresh = Instant::now();
                        return None;
                    };

                    let action = PackageAction::Install;
                    if matches!(app.pending_package_action.as_ref(),
                        Some(pending) if pending.action == action && pending.pkg == pkg)
                    {
                        app.request_command("install", &["install", &pkg], &channels.command_tx);
                        app.pending_package_action = None;
                        app.status = "Installing...".to_string();
                        app.last_refresh = Instant::now();
                    } else {
                        app.pending_package_action = Some(PendingPackageAction {
                            action,
                            pkg: pkg.clone(),
                        });
                        app.status = format!("Install {pkg}? [i] confirm, [Esc] cancel");
                        app.last_refresh = Instant::now();
                    }
                } else {
                    app.status = "Focus leaves to install".to_string();
                    app.last_refresh = Instant::now();
                }
            }
            KeyCode::Char('u') => {
                if app.focus_panel == FocusedPanel::Leaves {
                    let Some(pkg) = app.selected_leaf().map(str::to_string) else {
                        app.status = "No leaf selected".to_string();
                        app.last_refresh = Instant::now();
                        return None;
                    };

                    let action = PackageAction::Uninstall;
                    if matches!(app.pending_package_action.as_ref(),
                        Some(pending) if pending.action == action && pending.pkg == pkg)
                    {
                        app.request_command(
                            "uninstall",
                            &["uninstall", &pkg],
                            &channels.command_tx,
                        );
                        app.pending_package_action = None;
                        app.status = "Uninstalling...".to_string();
                        app.last_refresh = Instant::now();
                    } else {
                        app.pending_package_action = Some(PendingPackageAction {
                            action,
                            pkg: pkg.clone(),
                        });
                        app.status = format!("Uninstall {pkg}? [u] confirm, [Esc] cancel");
                        app.last_refresh = Instant::now();
                    }
                } else {
                    app.status = "Focus leaves to uninstall".to_string();
                    app.last_refresh = Instant::now();
                }
            }
            KeyCode::Char('c') => {
                app.request_command("cleanup", &["cleanup", "-s"], &channels.command_tx);
            }
            KeyCode::Char('a') => {
                app.request_command("autoremove", &["autoremove"], &channels.command_tx);
            }
            KeyCode::Char('b') => {
                app.request_command(
                    "bundle dump",
                    &["bundle", "dump", "--force"],
                    &channels.command_tx,
                );
            }
            KeyCode::Enter => {
                app.request_details(DetailsLoad::Basic, &channels.details_tx);
            }
            KeyCode::Char('d') => {
                app.request_details(DetailsLoad::Full, &channels.details_tx);
            }
            KeyCode::Tab => app.cycle_focus(),
            KeyCode::BackTab => {
                handle_focus_backtab(app);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.scroll_focused_up();
                if app.focus_panel == FocusedPanel::Leaves {
                    app.pending_package_action = None;
                    app.on_selection_change();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.scroll_focused_down();
                if app.focus_panel == FocusedPanel::Leaves {
                    app.pending_package_action = None;
                    app.on_selection_change();
                }
            }
            KeyCode::Left | KeyCode::Char('l') if app.focus_panel == FocusedPanel::Status => {
                app.health_tab_prev();
            }
            KeyCode::Right | KeyCode::Char(';') if app.focus_panel == FocusedPanel::Status => {
                app.health_tab_next();
            }
            _ => {}
        },
        InputMode::SearchLeaves => match key.code {
            KeyCode::Enter => {
                // Exit typing mode, filter persists, i/u now available
                app.input_mode = InputMode::Normal;
                app.status = "Ready".to_string();
                app.last_refresh = Instant::now();
            }
            KeyCode::Esc => {
                // Clear filter and exit
                if !app.leaves_query.is_empty() {
                    app.leaves_query.clear();
                    app.update_filtered_leaves();
                }
                app.input_mode = InputMode::Normal;
                app.status = "Ready".to_string();
                app.last_refresh = Instant::now();
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
                app.update_filtered_leaves();
            }
            KeyCode::Char(ch) => {
                app.leaves_query.push(ch);
                app.update_filtered_leaves();
            }
            _ => {}
        },
        InputMode::PackageSearch => match key.code {
            KeyCode::Esc => {
                // Cancel and exit to Normal
                app.input_mode = InputMode::Normal;
                app.package_query.clear();
                app.clear_package_results();
                app.status = "Ready".to_string();
                app.last_refresh = Instant::now();
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
                    app.status = "Enter a package name".to_string();
                    app.last_refresh = Instant::now();
                    return None;
                }

                app.request_command("search", &["search", &query], &channels.command_tx);
                app.last_package_search = Some(query);
                app.status = "Searching...".to_string();
                app.last_refresh = Instant::now();
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
        },
        InputMode::PackageResults => match key.code {
            KeyCode::Esc => {
                if app.pending_package_action.is_some() {
                    app.pending_package_action = None;
                    app.status = "Canceled".to_string();
                    app.last_refresh = Instant::now();
                } else {
                    app.input_mode = InputMode::Normal;
                    app.package_query.clear();
                    app.clear_package_results();
                    app.status = "Ready".to_string();
                    app.last_refresh = Instant::now();
                }
            }
            KeyCode::Char('f') => {
                // Go back to search input for a new query
                app.input_mode = InputMode::PackageSearch;
                app.package_query.clear();
                app.clear_package_results();
                app.pending_package_action = None;
                app.status = "Search packages".to_string();
                app.last_refresh = Instant::now();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.select_prev_result();
                app.pending_package_action = None;
                app.on_selection_change();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.select_next_result();
                app.pending_package_action = None;
                app.on_selection_change();
            }
            KeyCode::Char('i') => {
                let Some(pkg) = app.selected_package_result().map(str::to_string) else {
                    app.status = "No result selected".to_string();
                    app.last_refresh = Instant::now();
                    return None;
                };

                let action = PackageAction::Install;
                if matches!(app.pending_package_action.as_ref(),
                    Some(pending) if pending.action == action && pending.pkg == pkg)
                {
                    app.request_command("install", &["install", &pkg], &channels.command_tx);
                    app.pending_package_action = None;
                    app.status = "Installing...".to_string();
                    app.last_refresh = Instant::now();
                } else {
                    app.pending_package_action = Some(PendingPackageAction {
                        action,
                        pkg: pkg.clone(),
                    });
                    app.status = format!("Install {pkg}? [i] confirm, [Esc] cancel");
                    app.last_refresh = Instant::now();
                }
            }
            KeyCode::Char('u') => {
                let Some(pkg) = app.selected_package_result().map(str::to_string) else {
                    app.status = "No result selected".to_string();
                    app.last_refresh = Instant::now();
                    return None;
                };

                let action = PackageAction::Uninstall;
                if matches!(app.pending_package_action.as_ref(),
                    Some(pending) if pending.action == action && pending.pkg == pkg)
                {
                    app.request_command("uninstall", &["uninstall", &pkg], &channels.command_tx);
                    app.pending_package_action = None;
                    app.status = "Uninstalling...".to_string();
                    app.last_refresh = Instant::now();
                } else {
                    app.pending_package_action = Some(PendingPackageAction {
                        action,
                        pkg: pkg.clone(),
                    });
                    app.status = format!("Uninstall {pkg}? [u] confirm, [Esc] cancel");
                    app.last_refresh = Instant::now();
                }
            }
            _ => {}
        },
    }

    None
}
