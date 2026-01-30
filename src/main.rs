use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::app::{App, InputMode};
use crate::brew::DetailsLoad;
use crate::ui::draw;

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

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        while let Ok(message) = details_rx.try_recv() {
            app.apply_details_message(message);
        }

        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('r') => app.refresh_leaves(),
                            KeyCode::Char('t') => app.cycle_theme(),
                            KeyCode::Char('/') => {
                                app.input_mode = InputMode::Search;
                                app.search_query.clear();
                                app.status = "Search".to_string();
                                app.last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Enter => {
                                app.request_details(DetailsLoad::Basic, &details_tx);
                            }
                            KeyCode::Char('d') => {
                                app.request_details(DetailsLoad::Full, &details_tx);
                            }
                            KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                            _ => {}
                        },
                        InputMode::Search => match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                app.input_mode = InputMode::Normal;
                                app.status = "Ready".to_string();
                                app.last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                            }
                            KeyCode::Char(ch) => {
                                app.search_query.push(ch);
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
