use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, InputMode};
use crate::ui::util::icon_label;

pub fn draw_search_panel(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let search_icon = icon_label(app, "󰍉", "");
    let installed_label = app.active_kind_label_plural();

    // Padding (replace with smarter implementation soon)
    let title_prefix = if app.icons_ascii { "" } else { " " };
    let (label, is_active) = match app.input_mode {
        InputMode::SearchLeaves => (
            format!("{title_prefix}{search_icon} Search {installed_label}"),
            true,
        ),
        InputMode::PackageSearch => (format!("{title_prefix}{search_icon} Search packages"), true),
        InputMode::PackageResults => (
            format!("{title_prefix}{search_icon} Package results"),
            false,
        ),
        InputMode::Normal => (format!("{title_prefix}{search_icon} Search"), false),
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
        InputMode::PackageSearch | InputMode::PackageResults => &app.package_query,
        _ => &app.leaves_query,
    };

    let search_text = if search_value.is_empty() {
        if is_active {
            let hint = match app.input_mode {
                InputMode::PackageSearch => "type to search... (Enter to search, Esc to cancel)",
                InputMode::SearchLeaves => "type to filter... (Enter to browse, Esc to clear)",
                _ => "type to filter...",
            };
            Span::styled(hint, Style::default().fg(theme.text_muted))
        } else {
            Span::styled(
                "f package, / installed, o outdated-only",
                Style::default().fg(theme.text_muted),
            )
        }
    } else {
        Span::styled(search_value, Style::default().fg(theme.text_primary))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_panel))
        .title(Span::styled(label, Style::default().fg(title_color)));

    let inner = block.inner(area);
    frame.render_widget(block, area);
    let padded = Rect {
        x: inner.x.saturating_add(2),
        y: inner.y,
        width: inner.width.saturating_sub(2),
        height: inner.height,
    };

    let search = Paragraph::new(Line::from(vec![search_text]))
        .style(Style::default().bg(theme.bg_panel))
        .wrap(Wrap { trim: true });
    frame.render_widget(search, padded);
}
