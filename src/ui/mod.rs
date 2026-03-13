mod details;
mod footer;
pub mod help;
pub mod layout;
mod leaves;
mod search;
mod sizes;
mod status;
mod util;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::{App, FocusedPanel, StatusTab};
use crate::theme::ThemeMode;

pub fn status_tab_at_column(app: &App, area: Rect, column: u16) -> Option<StatusTab> {
    status::tab_at_column(app, area, column)
}

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let theme = &app.theme;
    let app_layout = layout::split_app(frame.area());

    let bg_block = Block::default().style(Style::default().bg(theme.bg_main));
    frame.render_widget(bg_block, frame.area());

    let dimmed = app.show_help_popup;
    draw_header(frame, app_layout.header, app, dimmed);
    draw_body(frame, app, app_layout);
    footer::draw_footer(frame, app_layout.footer, app, dimmed);

    if app.show_help_popup {
        help::draw_help_popup(frame, app);
    }
}

fn draw_header(frame: &mut ratatui::Frame, area: Rect, app: &App, dimmed: bool) {
    let theme = &app.theme;
    let uptime = app.started_at.elapsed().as_secs();
    let current_version = env!("CARGO_PKG_VERSION");

    let theme_indicator = match app.theme_mode {
        ThemeMode::Auto => "auto",
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    };

    let mut version_label = format!("v{current_version}");
    if let Some(status) = app.system_status.as_ref()
        && status.brewery_update_available
        && let Some(latest) = status.brewery_latest_version.as_ref()
    {
        version_label = format!("v{current_version} (update: v{latest})");
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

    let bg = if dimmed { theme.bg_dim } else { theme.bg_panel };
    let header = Paragraph::new(line).style(Style::default().bg(bg));
    frame.render_widget(header, area);
}

fn draw_body(frame: &mut ratatui::Frame, app: &App, layout: layout::AppLayout) {
    search::draw_search_panel(frame, layout.search, app);
    leaves::draw_leaves_panel(
        frame,
        layout.leaves,
        app,
        app.focus_panel == FocusedPanel::Leaves,
    );
    sizes::draw_sizes_panel(
        frame,
        layout.sizes,
        app,
        app.focus_panel == FocusedPanel::Sizes,
    );
    status::draw_status_panel(
        frame,
        layout.status,
        app,
        app.focus_panel == FocusedPanel::Status,
    );
    details::draw_details_panel(
        frame,
        layout.details,
        app,
        app.focus_panel == FocusedPanel::Details,
    );
}
