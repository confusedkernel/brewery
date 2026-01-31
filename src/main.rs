use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::app::{App, FocusedPanel, InputMode, ViewMode};
use crate::brew::DetailsLoad;
use crate::ui::{draw, help};

mod app;
mod brew;
mod theme;
mod ui;

const TICK_RATE: Duration = Duration::from_millis(250);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut terminal = setup_terminal()?;

    let result = run_app(&mut terminal).await;
    restore_terminal(&mut terminal)?;

    result
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let mut app = App::new();
    app.refresh_leaves();
    let (details_tx, mut details_rx) = mpsc::unbounded_channel();
    let (sizes_tx, mut sizes_rx) = mpsc::unbounded_channel();
    let (command_tx, mut command_rx) = mpsc::unbounded_channel();
    let (health_tx, mut health_rx) = mpsc::unbounded_channel();

    // Auto-fetch health and sizes on startup
    app.request_health(&health_tx);
    app.request_sizes(&sizes_tx);

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        while let Ok(message) = details_rx.try_recv() {
            app.apply_details_message(message);
        }
        while let Ok(message) = sizes_rx.try_recv() {
            app.apply_sizes_message(message);
        }
        while let Ok(message) = command_rx.try_recv() {
            app.apply_command_message(message);
        }
        while let Ok(message) = health_rx.try_recv() {
            app.apply_health_message(message);
        }

        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Handle help popup toggle first
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
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('r') => app.refresh_leaves(),
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
                                app.status = "Search packages".to_string();
                                app.last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Char('i') => {
                                app.input_mode = InputMode::PackageInstall;
                                app.package_query.clear();
                                app.status = "Install package".to_string();
                                app.last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Char('u') => {
                                app.input_mode = InputMode::PackageUninstall;
                                app.package_query.clear();
                                app.status = "Uninstall package".to_string();
                                app.last_refresh = std::time::Instant::now();
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
                            KeyCode::Up | KeyCode::Char('k') => app.scroll_focused_up(),
                            KeyCode::Down | KeyCode::Char('j') => app.scroll_focused_down(),
                            KeyCode::Left | KeyCode::Char('l') if app.focus_panel == FocusedPanel::Health => {
                                app.health_tab_prev();
                            }
                            KeyCode::Right | KeyCode::Char(';') if app.focus_panel == FocusedPanel::Health => {
                                app.health_tab_next();
                            }
                            _ => {}
                        },
                        InputMode::SearchLeaves => match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                app.input_mode = InputMode::Normal;
                                app.status = "Ready".to_string();
                                app.last_refresh = std::time::Instant::now();
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
                        InputMode::PackageSearch
                        | InputMode::PackageInstall
                        | InputMode::PackageUninstall => match key.code {
                            KeyCode::Esc => {
                                app.input_mode = InputMode::Normal;
                                app.status = "Ready".to_string();
                                app.last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Enter => {
                                let query = app.package_query.trim().to_string();
                                if query.is_empty() {
                                    app.status = "Enter a package name".to_string();
                                    app.last_refresh = std::time::Instant::now();
                                    continue;
                                }

                                match app.input_mode {
                                    InputMode::PackageSearch => {
                                        app.request_command(
                                            "search",
                                            &["search", &query],
                                            &command_tx,
                                        );
                                        app.view_mode = crate::app::ViewMode::PackageResults;
                                        app.status = "Search submitted".to_string();
                                        app.last_refresh = std::time::Instant::now();
                                        continue;
                                    }
                                    InputMode::PackageInstall => {
                                        app.request_command(
                                            "install",
                                            &["install", &query],
                                            &command_tx,
                                        );
                                    }
                                    InputMode::PackageUninstall => {
                                        app.request_command(
                                            "uninstall",
                                            &["uninstall", &query],
                                            &command_tx,
                                        );
                                    }
                                    _ => {}
                                }

                                app.input_mode = InputMode::Normal;
                            }
                            KeyCode::Backspace => {
                                app.package_query.pop();
                            }
                            KeyCode::Char(ch) => {
                                app.package_query.push(ch);
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
