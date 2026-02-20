use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn draw_footer(frame: &mut ratatui::Frame, area: Rect, app: &App, dimmed: bool) {
    let theme = &app.theme;

    let keys = vec![
        ("q", "quit"),
        ("tab", "tabs"),
        ("i", "install"),
        ("u", "uninstall"),
        ("U", "upgrade"),
        ("f,/", "search"),
        ("?", "help"),
    ];

    let mut spans = Vec::new();
    for (key, label) in keys {
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(theme.text_on_accent)
                .bg(theme.accent_secondary),
        ));
        spans.push(Span::styled(
            format!(" {} ", label),
            Style::default().fg(theme.text_secondary),
        ));
    }

    let mut all_spans = vec![Span::raw(" ")];
    all_spans.extend(spans);
    let line = Line::from(all_spans);
    let bg = if dimmed { theme.bg_dim } else { theme.bg_panel };
    let footer = Paragraph::new(line).style(Style::default().bg(bg));
    frame.render_widget(footer, area);
}
