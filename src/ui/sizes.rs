use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::format::format_size;
use crate::ui::util::{panel_block, panel_title, styled_line};

pub fn draw_sizes_panel(frame: &mut ratatui::Frame, area: Rect, app: &App, is_focused: bool) {
    let theme = &app.theme;

    let title = if app.sizes_job.is_running() {
        " Sizes (loading...)"
    } else {
        " Sizes"
    };

    let lines = if app.sizes.is_empty() {
        vec![
            Line::from(""),
            styled_line("  Press 's' to load sizes", theme.text_muted),
        ]
    } else {
        app.sizes
            .iter()
            .take(20)
            .skip(app.sizes_scroll_offset)
            .take(8)
            .map(|entry| {
                Line::from(vec![
                    Span::styled(
                        format!("  {:>6}", format_size(entry.size_kb)),
                        Style::default().fg(theme.yellow),
                    ),
                    Span::styled(
                        format!("  {}", entry.name),
                        Style::default().fg(theme.text_primary),
                    ),
                ])
            })
            .collect()
    };

    let block = panel_block(
        app,
        panel_title(title, theme.yellow, is_focused),
        is_focused,
    );

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_panel));
    frame.render_widget(paragraph, area);
}
