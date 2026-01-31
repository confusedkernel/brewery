use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, InputMode};
use crate::ui::util::icon_label;

pub fn draw_search_panel(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let search_icon = icon_label(app, "󰍉", "");
    let install_icon = icon_label(app, "󰏗", "+");
    let remove_icon = icon_label(app, "󰆴", "-");

    let (label, is_active) = match app.input_mode {
        InputMode::SearchLeaves => (format!("{search_icon} Search leaves"), true),
        InputMode::PackageSearch => (format!("{search_icon} Search packages"), true),
        InputMode::PackageInstall => (format!("{install_icon} Install package"), true),
        InputMode::PackageUninstall => (format!("{remove_icon} Remove package"), true),
        InputMode::Normal => (format!("{search_icon} Search (/)"), false),
    };

    let border_color = if is_active {
        theme.border_active
    } else {
        theme.border
    };
    let title_color = if is_active {
        theme.accent
    } else {
        theme.text_secondary
    };

    let search_value = match app.input_mode {
        InputMode::PackageSearch | InputMode::PackageInstall | InputMode::PackageUninstall => {
            &app.package_query
        }
        _ => &app.leaves_query,
    };

    let search_text = if search_value.is_empty() {
        Span::styled("type to filter...", Style::default().fg(theme.text_muted))
    } else {
        Span::styled(search_value, Style::default().fg(theme.text_primary))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_panel))
        .title(Span::styled(label, Style::default().fg(title_color)));

    let search = Paragraph::new(Line::from(vec![search_text]))
        .block(block)
        .style(Style::default().bg(theme.bg_panel))
        .wrap(Wrap { trim: true });
    frame.render_widget(search, area);
}
