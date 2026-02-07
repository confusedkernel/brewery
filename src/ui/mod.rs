mod details;
mod footer;
pub mod help;
mod leaves;
mod search;
mod sizes;
mod status;
mod util;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::{App, FocusedPanel};
use crate::theme::ThemeMode;

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let theme = &app.theme;

    let bg_block = Block::default().style(Style::default().bg(theme.bg_main));
    frame.render_widget(bg_block, frame.area());

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, layout[0], app);
    draw_body(frame, layout[1], app);
    footer::draw_footer(frame, layout[2], app);

    if app.show_help_popup {
        help::draw_help_popup(frame, app);
    }
}

fn draw_header(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let uptime = app.started_at.elapsed().as_secs();
    let current_version = env!("CARGO_PKG_VERSION");

    let theme_indicator = match app.theme_mode {
        ThemeMode::Auto => "auto",
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    };

    let mut version_label = format!("v{current_version}");
    if let Some(status) = app.system_status.as_ref() {
        if status.brewery_update_available {
            if let Some(latest) = status.brewery_latest_version.as_ref() {
                version_label = format!("v{current_version} (update: v{latest})");
            }
        }
    }

    let version_color = if app
        .system_status
        .as_ref()
        .map(|status| status.brewery_update_available)
        .unwrap_or(false)
    {
        theme.orange
    } else {
        theme.text_muted
    };

    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            " brewery ",
            Style::default()
                .fg(theme.text_on_accent)
                .bg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(version_label, Style::default().fg(version_color)),
        Span::raw("  "),
        Span::styled(&app.status, Style::default().fg(theme.text_secondary)),
        Span::raw("  "),
        Span::styled(
            format!("[{}] {}s", theme_indicator, uptime),
            Style::default().fg(theme.text_secondary),
        ),
    ]);

    let header = Paragraph::new(line).style(Style::default().bg(theme.bg_panel));
    frame.render_widget(header, area);
}

fn draw_body(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(10),
        ])
        .split(columns[0]);

    let right_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(columns[1]);

    search::draw_search_panel(frame, left_panels[0], app);
    leaves::draw_leaves_panel(
        frame,
        left_panels[1],
        app,
        app.focus_panel == FocusedPanel::Leaves,
    );
    sizes::draw_sizes_panel(
        frame,
        left_panels[2],
        app,
        app.focus_panel == FocusedPanel::Sizes,
    );
    status::draw_status_panel(
        frame,
        right_panels[0],
        app,
        app.focus_panel == FocusedPanel::Status,
    );
    details::draw_details_panel(
        frame,
        right_panels[1],
        app,
        app.focus_panel == FocusedPanel::Details,
    );
}
