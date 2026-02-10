use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::size;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::App;
use crate::runtime::input::handle_key_event;
use crate::runtime::messages::{create_channels, handle_auto_details, process_pending_messages};
use crate::ui::{draw, help};

/// Poll interval while spinner/progress is active.
const ACTIVE_TICK_RATE: Duration = Duration::from_millis(80);

/// Maximum poll interval while app is idle.
const IDLE_TICK_RATE: Duration = Duration::from_secs(1);

/// Debounce delay for auto-fetching details when selection changes.
/// This prevents rapid-fire requests when scrolling quickly through lists.
/// Details will only be fetched after the user has stopped on an item for this duration.
const DETAILS_DEBOUNCE: Duration = Duration::from_millis(300);

pub async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let mut app = App::new();
    let mut channels = create_channels();
    let mut last_uptime_second = app.started_at.elapsed().as_secs();

    let mut last_fetched_leaf: Option<String> = None;

    // Kick off all startup fetches in parallel (non-blocking)
    app.request_leaves(&channels.leaves_tx);
    app.request_status(&channels.status_tx);
    app.request_sizes(&channels.sizes_tx);

    loop {
        let current_uptime_second = app.started_at.elapsed().as_secs();
        if current_uptime_second != last_uptime_second {
            app.needs_redraw = true;
            last_uptime_second = current_uptime_second;
        }

        // Only redraw when needed
        if app.needs_redraw {
            terminal.draw(|frame| draw(frame, &app))?;
            app.needs_redraw = false;
        }

        process_pending_messages(&mut app, &mut channels);

        // Debounced auto-fetch details for package search results
        // Skip if user is rapidly scrolling to reduce CPU load
        handle_auto_details(
            &mut app,
            &mut last_fetched_leaf,
            &channels.details_tx,
            DETAILS_DEBOUNCE,
        );

        let tick_rate = if app.pending_command {
            ACTIVE_TICK_RATE
        } else {
            IDLE_TICK_RATE
        };

        if event::poll(tick_rate)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            let max_offset = help_max_offset(&app);
            if let Some(result) = handle_key_event(&mut app, key, &channels, max_offset) {
                return result;
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
