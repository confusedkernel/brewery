use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::app::{App, FocusedPanel, InputMode, PackageAction, PendingPackageAction, ViewMode};
use crate::brew::{DetailsLoad, LeavesMessage};
use crate::ui::{draw, help};

mod app;
mod brew;
mod theme;
mod ui;

/// Tick rate for the main event loop (500ms for good balance of responsiveness and CPU)
const TICK_RATE: Duration = Duration::from_millis(500);

/// Debounce delay for auto-fetching details when selection changes.
/// This prevents rapid-fire requests when scrolling quickly through lists.
/// Details will only be fetched after the user has stopped on an item for this duration.
const DETAILS_DEBOUNCE: Duration = Duration::from_millis(400);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut terminal = setup_terminal()?;

    let result = run_app(&mut terminal).await;
    restore_terminal(&mut terminal)?;

    result
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let mut app = App::new();
    
    // Create all message channels
    let (leaves_tx, mut leaves_rx) = mpsc::unbounded_channel::<LeavesMessage>();
    let (details_tx, mut details_rx) = mpsc::unbounded_channel();
    let (sizes_tx, mut sizes_rx) = mpsc::unbounded_channel();
    let (command_tx, mut command_rx) = mpsc::unbounded_channel();
    let (health_tx, mut health_rx) = mpsc::unbounded_channel();
    
    let mut last_selected_leaf: Option<String> = None;

    // Kick off all startup fetches in parallel (non-blocking)
    app.request_leaves(&leaves_tx);
    app.request_health(&health_tx);
    app.request_sizes(&sizes_tx);

    loop {
        // Only redraw when needed
        if app.needs_redraw {
            terminal.draw(|frame| draw(frame, &app))?;
            app.needs_redraw = false;
        }

        // Process all pending messages (mark dirty on each)
        let mut received_message = false;
        
        while let Ok(message) = leaves_rx.try_recv() {
            app.apply_leaves_message(message);
            received_message = true;
        }
        while let Ok(message) = details_rx.try_recv() {
            app.apply_details_message(message);
            received_message = true;
        }
        while let Ok(message) = sizes_rx.try_recv() {
            app.apply_sizes_message(message);
            received_message = true;
        }
        while let Ok(message) = command_rx.try_recv() {
            app.apply_command_message(message);
            received_message = true;
        }
        while let Ok(message) = health_rx.try_recv() {
            app.apply_health_message(message);
            received_message = true;
        }

        // If we received messages, request a redraw
        if received_message {
            app.needs_redraw = true;
        }

        // Debounced auto-fetch details for package search results
        if matches!(app.input_mode, InputMode::PackageSearch | InputMode::PackageResults) {
            if let Some(pkg) = app.selected_package_result().map(str::to_string) {
                if app.last_result_details_pkg.as_deref() != Some(pkg.as_str()) {
                    // Check debounce
                    let should_fetch = app.last_selection_change
                        .map(|t| t.elapsed() >= DETAILS_DEBOUNCE)
                        .unwrap_or(true);
                    
                    if should_fetch {
                        app.request_details_for(&pkg, DetailsLoad::Basic, &details_tx);
                        app.last_result_details_pkg = Some(pkg);
                    }
                }
            }
        }

        // Debounced auto-fetch details for selected leaf
        if !matches!(app.input_mode, InputMode::PackageSearch | InputMode::PackageResults) {
            let selected = app.selected_leaf().map(str::to_string);
            if selected.is_some() && selected != last_selected_leaf {
                // Check debounce
                let should_fetch = app.last_selection_change
                    .map(|t| t.elapsed() >= DETAILS_DEBOUNCE)
                    .unwrap_or(true);
                
                if should_fetch {
                    app.request_details(DetailsLoad::Basic, &details_tx);
                    last_selected_leaf = selected;
                }
            }
        }

        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Any keypress should trigger a redraw
                    app.needs_redraw = true;
                    
                    // Handle global keymaps only in Normal mode
                    if app.input_mode == InputMode::Normal {
                        if key.code == KeyCode::Char('?') {
                            app.toggle_help();
                            continue;
                        }

                        if key.code == KeyCode::Char('i')
                            && key.modifiers.contains(KeyModifiers::ALT)
                        {
                            app.toggle_icons();
                            continue;
                        }
                    }

                    // Close help popup with Esc
                    if app.show_help_popup {
                        match key.code {
                            KeyCode::Esc => {
                                app.show_help_popup = false;
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let max_offset = help_max_offset(&app);
                                if app.help_scroll_offset < max_offset {
                                    app.help_scroll_offset += 1;
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.help_scroll_offset = app.help_scroll_offset.saturating_sub(1);
                            }
                            _ => {}
                        }
                        continue;
                    }

                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') => return Ok(()),
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
                            KeyCode::Char('r') => app.request_leaves(&leaves_tx),
                            KeyCode::Char('t') => app.cycle_theme(),
                            KeyCode::Char('s') => app.request_sizes(&sizes_tx),
                            KeyCode::Char('h') => app.request_health(&health_tx),
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
                                        continue;
                                    };

                                    let action = PackageAction::Install;
                                    if matches!(app.pending_package_action.as_ref(),
                                        Some(pending) if pending.action == action && pending.pkg == pkg)
                                    {
                                        app.request_command(
                                            "install",
                                            &["install", &pkg],
                                            &command_tx,
                                        );
                                        app.pending_package_action = None;
                                        app.status = "Installing...".to_string();
                                        app.last_refresh = Instant::now();
                                    } else {
                                        app.pending_package_action = Some(PendingPackageAction {
                                            action,
                                            pkg: pkg.clone(),
                                        });
                                        app.status =
                                            format!("Install {pkg}? [i] confirm, [Esc] cancel");
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
                                        continue;
                                    };

                                    let action = PackageAction::Uninstall;
                                    if matches!(app.pending_package_action.as_ref(),
                                        Some(pending) if pending.action == action && pending.pkg == pkg)
                                    {
                                        app.request_command(
                                            "uninstall",
                                            &["uninstall", &pkg],
                                            &command_tx,
                                        );
                                        app.pending_package_action = None;
                                        app.status = "Uninstalling...".to_string();
                                        app.last_refresh = Instant::now();
                                    } else {
                                        app.pending_package_action = Some(PendingPackageAction {
                                            action,
                                            pkg: pkg.clone(),
                                        });
                                        app.status =
                                            format!("Uninstall {pkg}? [u] confirm, [Esc] cancel");
                                        app.last_refresh = Instant::now();
                                    }
                                } else {
                                    app.status = "Focus leaves to uninstall".to_string();
                                    app.last_refresh = Instant::now();
                                }
                            }
                            KeyCode::Char('c') => {
                                app.request_command("cleanup", &["cleanup", "-s"], &command_tx);
                            }
                            KeyCode::Char('a') => {
                                app.request_command("autoremove", &["autoremove"], &command_tx);
                            }
                            KeyCode::Char('b') => {
                                app.request_command(
                                    "bundle dump",
                                    &["bundle", "dump", "--force"],
                                    &command_tx,
                                );
                            }
                            KeyCode::Enter => {
                                app.request_details(DetailsLoad::Basic, &details_tx);
                            }
                            KeyCode::Char('d') => {
                                app.request_details(DetailsLoad::Full, &details_tx);
                            }
                            KeyCode::Tab => app.cycle_focus(),
                            KeyCode::BackTab => {
                                // Shift+Tab: cycle backwards
                                app.focus_panel = match app.focus_panel {
                                    FocusedPanel::Leaves => FocusedPanel::Details,
                                    FocusedPanel::Sizes => FocusedPanel::Leaves,
                                    FocusedPanel::Health => FocusedPanel::Sizes,
                                    FocusedPanel::Details => FocusedPanel::Health,
                                };
                                app.status = format!("Focus: {:?}", app.focus_panel);
                                app.last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.scroll_focused_up();
                                if app.focus_panel == FocusedPanel::Leaves {
                                    app.pending_package_action = None;
                                    app.last_selection_change = Some(Instant::now());
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.scroll_focused_down();
                                if app.focus_panel == FocusedPanel::Leaves {
                                    app.pending_package_action = None;
                                    app.last_selection_change = Some(Instant::now());
                                }
                            }
                            KeyCode::Left | KeyCode::Char('l') if app.focus_panel == FocusedPanel::Health => {
                                app.health_tab_prev();
                            }
                            KeyCode::Right | KeyCode::Char(';') if app.focus_panel == FocusedPanel::Health => {
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
                                app.last_selection_change = Some(Instant::now());
                            }
                            KeyCode::Down => {
                                app.select_next();
                                app.last_selection_change = Some(Instant::now());
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
                                app.last_selection_change = Some(Instant::now());
                            }
                            KeyCode::Down => {
                                app.select_next_result();
                                app.last_selection_change = Some(Instant::now());
                            }
                            KeyCode::Enter => {
                                let query = app.package_query.trim().to_string();
                                if query.is_empty() {
                                    app.status = "Enter a package name".to_string();
                                    app.last_refresh = Instant::now();
                                    continue;
                                }

                                app.request_command(
                                    "search",
                                    &["search", &query],
                                    &command_tx,
                                );
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
                                app.last_selection_change = Some(Instant::now());
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.select_next_result();
                                app.pending_package_action = None;
                                app.last_selection_change = Some(Instant::now());
                            }
                            KeyCode::Char('i') => {
                                let Some(pkg) =
                                    app.selected_package_result().map(str::to_string)
                                else {
                                    app.status = "No result selected".to_string();
                                    app.last_refresh = Instant::now();
                                    continue;
                                };

                                let action = PackageAction::Install;
                                if matches!(app.pending_package_action.as_ref(),
                                    Some(pending) if pending.action == action && pending.pkg == pkg)
                                {
                                    app.request_command("install", &["install", &pkg], &command_tx);
                                    app.pending_package_action = None;
                                    app.status = "Installing...".to_string();
                                    app.last_refresh = Instant::now();
                                } else {
                                    app.pending_package_action = Some(PendingPackageAction {
                                        action,
                                        pkg: pkg.clone(),
                                    });
                                    app.status =
                                        format!("Install {pkg}? [i] confirm, [Esc] cancel");
                                    app.last_refresh = Instant::now();
                                }
                            }
                            KeyCode::Char('u') => {
                                let Some(pkg) =
                                    app.selected_package_result().map(str::to_string)
                                else {
                                    app.status = "No result selected".to_string();
                                    app.last_refresh = Instant::now();
                                    continue;
                                };

                                let action = PackageAction::Uninstall;
                                if matches!(app.pending_package_action.as_ref(),
                                    Some(pending) if pending.action == action && pending.pkg == pkg)
                                {
                                    app.request_command(
                                        "uninstall",
                                        &["uninstall", &pkg],
                                        &command_tx,
                                    );
                                    app.pending_package_action = None;
                                    app.status = "Uninstalling...".to_string();
                                    app.last_refresh = Instant::now();
                                } else {
                                    app.pending_package_action = Some(PendingPackageAction {
                                        action,
                                        pkg: pkg.clone(),
                                    });
                                    app.status =
                                        format!("Uninstall {pkg}? [u] confirm, [Esc] cancel");
                                    app.last_refresh = Instant::now();
                                }
                            }
                            _ => {}
                        },
                    }
                }
            }
        }

        app.on_tick();
    }
}

fn help_max_offset(app: &App) -> usize {
    let (_, rows) = size().unwrap_or((0, 0));
    let popup_height = 22u16.min(rows.saturating_sub(4));
    let visible_height = popup_height.saturating_sub(2) as usize;
    let total_lines = help::help_line_count(app);
    total_lines.saturating_sub(visible_height)
}

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
