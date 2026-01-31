use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::util::format_size;

pub fn draw_sizes_panel(frame: &mut ratatui::Frame, area: Rect, app: &App, is_focused: bool) {
    let theme = &app.theme;

    let title = if app.pending_sizes {
        " Sizes (loading...)".to_string()
    } else {
        " Sizes".to_string()
    };

    let border_color = if is_focused {
        theme.border_active
    } else {
        theme.border
    };
    let title_modifier = if is_focused {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };

    let lines = if app.sizes.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Press 's' to load sizes",
                Style::default().fg(theme.text_muted),
            )),
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

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_panel))
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.yellow)
                .add_modifier(title_modifier),
        ));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_panel));
    frame.render_widget(paragraph, area);
}
